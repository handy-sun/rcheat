use std::convert::TryFrom;
use std::io;
use std::time::Instant;

use anyhow::{anyhow, Error};

use goblin::elf::{header, sym, Elf, SectionHeaders};
use goblin::strtab::Strtab;
use goblin::{Hint, Object};

use symbolic_common::Name;
use symbolic_demangle::{Demangle, DemangleOptions};

use regex::Regex;

use once_cell::sync::Lazy;

const MAGIC_LEN: usize = 16;

const DEM_OPT: DemangleOptions = DemangleOptions::name_only().parameters(true);

static RE_VAR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(anonymous namespace)|@GLIBC|std::|_IO_stdin_used|^\._|^__gnu_|^__cxxabiv|^guard variable|\)::__func__$|\.\d+$").unwrap()
});

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SymEntry {
    obj_addr: u64,
    obj_size: u64,
    bind_type: u8,
    origin_name: String,
    section: String,
}

impl SymEntry {
    pub fn obj_addr(&self) -> u64 {
        self.obj_addr
    }

    pub fn obj_size(&self) -> u64 {
        self.obj_size
    }
}

pub struct ElfMgr<'a> {
    elf: Elf<'a>,
}

impl<'a> ElfMgr<'a> {
    pub fn prase_from(bytes: &'a [u8]) -> Result<Self, Error> {
        let prefix_bytes_ref = bytes
            .get(..MAGIC_LEN)
            .ok_or_else(|| anyhow!("File size is too small: {} bytes", bytes.len()))?;
        let prefix_bytes = <&[u8; MAGIC_LEN]>::try_from(prefix_bytes_ref)?;

        if let Hint::Unknown(magic) = goblin::peek_bytes(prefix_bytes)? {
            return Err(anyhow!("Unknown magic: {:#x}", magic));
        }

        match Object::parse(bytes).unwrap() {
            Object::Elf(val) => Ok(ElfMgr { elf: val }),
            _ => Err(anyhow!("Object format not support")),
        }
    }

    pub fn is_exec_elf(&self) -> bool {
        self.elf.header.e_type == header::ET_EXEC
    }

    pub fn is_dyn_elf(&self) -> bool {
        self.elf.header.e_type == header::ET_DYN
    }

    pub fn select_sym_entry(&self, keyword: &String) -> Result<SymEntry, Error> {
        let start = Instant::now();
        let strtab = &self.elf.strtab;
        let syms = self.elf.syms.to_vec();
        // let dyn_strtab = &self.elf.dynstrtab;
        // let dynsyms = self.elf.dynsyms.to_vec();
        if syms.is_empty() {
            return Err(anyhow!("syms is empty"));
        }

        let map_iter = syms
            .iter()
            .filter_map(|sym| self.filter_symbol(sym, strtab, keyword));

        let emtry_vec: Vec<SymEntry> = map_iter.collect();
        println!("[{:?}] Time of `filter_symbol`", start.elapsed());

        match emtry_vec.len() {
            0 => Err(anyhow!("cannot find")),
            1 => {
                let entry = emtry_vec.first().unwrap().clone();
                println!("Matched var: {}", entry.origin_name);
                Ok(entry)
            }
            2.. => {
                println!("Matched count: {}", emtry_vec.len());
                println!("index: {:40} | var_size(B)", "var_name");
                for (i, emtry) in emtry_vec.iter().enumerate() {
                    println!("{:5}: {:40} | {:7}", i, emtry.origin_name, emtry.obj_size,);
                }
                loop_inquire_index(&emtry_vec)
            }
        }
    }

    fn filter_symbol(&self, sym: &sym::Sym, strtab: &Strtab, keyword: &String) -> Option<SymEntry> {
        // filter: LOCAL&OBJECT or GLOBAL&OBJECT
        if sym.st_type() != sym::STT_OBJECT
            || (sym.st_bind() != sym::STB_LOCAL && sym.st_bind() != sym::STB_GLOBAL)
        {
            return None;
        }

        let sym_symbol = strtab.get_at(sym.st_name).unwrap_or("BAD NAME");
        let name = Name::from(sym_symbol);
        let dem_name = name.try_demangle(DEM_OPT);
        let shn = shndx_to_str(sym.st_shndx, &self.elf.section_headers, &self.elf.shdr_strtab);

        if sym.st_size == 0
            || (!shn.starts_with(".bss(") && !shn.starts_with(".rodata(") && !shn.starts_with(".data"))
        {
            return None;
        }

        #[cfg(debug_assertions)]
        eprintln!(
            "{:6} | {:5} | {:16} | {:40} | {}",
            sym::bind_to_str(sym.st_bind()),
            sym.st_size,
            shn,
            dem_name,
            sym_symbol
        );
        if RE_VAR.is_match(&dem_name) {
            return None;
        }

        if dem_name.contains(keyword) || keyword.is_empty() {
            return Some(SymEntry {
                obj_addr: sym.st_value,
                obj_size: sym.st_size,
                bind_type: sym.st_bind(),
                origin_name: dem_name.to_string(),
                section: shn.clone(),
            });
        }
        None
    }
}

fn shndx_to_str(idx: usize, shdrs: &SectionHeaders, strtab: &Strtab) -> String {
    if idx == 0 {
        String::from("")
    } else if let Some(shdr) = shdrs.get(idx) {
        if let Some(link_name) = strtab.get_at(shdr.sh_name) {
            format!("{}({})", link_name, idx)
        } else {
            format!("BAD_IDX={}", shdr.sh_name)
        }
    } else if idx == 0xfff1 {
        // Associated symbol is absolute.
        String::from("ABS")
    } else {
        String::from(&format!("BAD_IDX={}", idx))
    }
}

/// the slice's len better greater than 0
fn loop_inquire_index<T>(entry_slice: &[T]) -> Result<T, Error>
where
    T: Clone,
{
    if entry_slice.is_empty() {
        return Err(anyhow!("The slice is empty"));
    }
    println!("Please input index to choose the var(default is 0): ");
    let mut line_input = String::with_capacity(16);
    loop {
        line_input.clear();
        match io::stdin().read_line(&mut line_input) {
            Ok(byte_count) => {
                // eprintln!("{} bytes read", byte_count);
                if let 0 = byte_count {
                    println!("read_line Ok and 0 byte read means EndOfFile");
                    return Err(anyhow!("EOF"));
                }
                let trimmed_input = line_input.trim();
                match trimmed_input.parse::<usize>() {
                    Ok(index) => {
                        if index < entry_slice.len() {
                            return Ok(entry_slice[index].clone());
                        } else {
                            println!(
                                "Input: {} out of range(available is [0, {}]): ",
                                index,
                                entry_slice.len() - 1
                            );
                        }
                    }
                    Err(parse_err) => {
                        if trimmed_input.is_empty() {
                            return Ok(entry_slice[0].clone());
                        }
                        println!("{:?} '{}', try again: ", parse_err, trimmed_input);
                    }
                }
            }
            Err(std_error) => println!("Failed to read line: {:?}", std_error),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_regex_of_var() {
        assert!(RE_VAR.is_match("completed.8061"));
        assert!(RE_VAR.is_match("._93"));
        assert!(RE_VAR.is_match("._anon_"));
        assert!(RE_VAR.is_match("..(anonymous namespace))abc"));
        assert!(RE_VAR.is_match("_IO_stdin_used"));
        assert!(RE_VAR.is_match("__gnu_@GLIBC"));
        assert!(RE_VAR.is_match("Cm::init()::__func__"));
    }
}
