use std::convert::TryFrom;
use std::io;

use anyhow::{anyhow, Error};

use goblin::elf::{header, sym, Elf, SectionHeaders};
use goblin::strtab::Strtab;
use goblin::{Hint, Object};

const MAGIC_LEN: usize = 16;

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
            Object::Elf(val) => Ok(ElfMgr {
                elf: val,
                // satisfied_entry: SymEntry::default(),
            }),
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
        println!("emtry_vec.len: {}", emtry_vec.len());
        match emtry_vec.len() {
            0 => Err(anyhow!("cannot find")),
            1 => Ok(emtry_vec[0].clone()),
            2.. => {
                for i in 0..emtry_vec.len() {
                    let emtry = &emtry_vec[i];
                    println!(
                        "{}: {:40} | {:7} | {:7} | {}",
                        i,
                        emtry.origin_name,
                        emtry.obj_size,
                        emtry.section,
                        sym::bind_to_str(emtry.bind_type),
                    );
                }

                let mut line_input = String::new();
                println!("Please input index to choose the var: ");
                // read from tty input
                let mut index: usize;
                loop {
                    io::stdin()
                        .read_line(&mut line_input)
                        .expect("Failed to read line");

                    index = line_input.trim().parse().expect("Index entered was not a number");

                    if index < emtry_vec.len() {
                        break;
                    }
                    println!("Input: {} out of range, try again: ", index);
                }
                Ok(emtry_vec[index].clone())
            }
        }
    }

    fn filter_symbol(&self, sym: &sym::Sym, strtab: &Strtab, keyword: &String) -> Option<SymEntry> {
        if sym.st_type() != sym::STT_OBJECT
            || (sym.st_bind() != sym::STB_LOCAL && sym.st_bind() != sym::STB_GLOBAL)
        {
            return None;
        }

        let name = strtab.get_at(sym.st_name).unwrap_or("BAD NAME");
        // assert_eq!(Name::from(name).detect_language(), Language::Cpp);
        let dem_name = union_demangle(name);

        let shn = shndx_to_str(sym.st_shndx, &self.elf.section_headers, &self.elf.shdr_strtab);

        if (
            shn.eq(".bss")
            // || shn.eq(".rodata")
        ) && !(dem_name.contains("@GLIBC")
            || dem_name.contains("_stdin")
            || dem_name.contains("_stdout")
            || dem_name.contains("anonymous")
            || dem_name.starts_with("__")
            || dem_name.starts_with("_dl")
            || dem_name.starts_with("_nl")
            || dem_name.starts_with("std::"))
            && sym.st_size > 0
        {
            #[cfg(debug_assertions)]
            println!(
                "{:7} | {:7} | {:8} | {:70} |",
                sym::bind_to_str(sym.st_bind()),
                sym.st_size,
                shn,
                dem_name
            );
            if dem_name.contains(keyword) {
                return Some(SymEntry {
                    obj_addr: sym.st_value,
                    obj_size: sym.st_size,
                    bind_type: sym.st_bind(),
                    origin_name: dem_name.clone(),
                    section: shn.clone(),
                });
            }
        }
        None
    }
}

fn shndx_to_str(idx: usize, shdrs: &SectionHeaders, strtab: &Strtab) -> String {
    if idx == 0 {
        String::from("")
    } else if let Some(shdr) = shdrs.get(idx) {
        if let Some(link_name) = strtab.get_at(shdr.sh_name).map(move |s| union_demangle(s)) {
            // format!("{}({})", link_name, idx)
            link_name
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

fn union_demangle(s: &str) -> String {
    match cpp_demangle::Symbol::new(s) {
        Ok(_symbol) => _symbol.to_string(),
        Err(_) => match rustc_demangle::try_demangle(s) {
            Ok(demangled) => demangled.to_string(),
            Err(_) => s.to_string(),
        },
    }
}
