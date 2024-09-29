use std::borrow::Cow;
use std::convert::TryFrom;
use std::fs::File;
use std::io;
use std::time::Instant;

use anyhow::{anyhow, Error};

use goblin::elf::{header, sym, Elf, SectionHeaders};
use goblin::strtab::Strtab;
use goblin::{Hint, Object};

use symbolic_common::{Language, Name};
use symbolic_demangle::{Demangle, DemangleOptions};

use regex::Regex;

use once_cell::sync::Lazy;

use crate::elf::DwarfInfoMatcher;

const MAGIC_LEN: usize = 16;

const DEM_OPT: DemangleOptions = DemangleOptions::name_only().parameters(true);

static RE_VAR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(anonymous namespace)|@GLIBC|std::|_IO_stdin_used|^\._|^__gnu_|^__cxxabiv|^guard variable|\)::__func__$|\.\d+$").unwrap()
});

/// Symbol (.symtab) entry only include the info we needed
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct SymEntry<'a> {
    obj_addr: u64,
    obj_size: u64,
    bind_type: u8,
    origin_name: String,
    mangled_name: Option<&'a str>,
    section: Cow<'a, str>,
}

impl<'a> SymEntry<'a> {
    pub fn obj_addr(&self) -> u64 {
        self.obj_addr
    }

    pub fn obj_size(&self) -> u64 {
        self.obj_size
    }

    pub fn is_local_bind(&self) -> bool {
        self.bind_type == sym::STB_LOCAL
    }
}

#[allow(dead_code)]
pub struct ElfMgr<'a> {
    elf: Elf<'a>,
    dw_matcher: DwarfInfoMatcher<'a>,
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
                dw_matcher: match DwarfInfoMatcher::parse(bytes) {
                    Ok(dw) => dw,
                    Err(_e) => return Err(anyhow!("Parse dwarf-sections failed: {:?}", _e)),
                },
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
        let start = Instant::now();
        let syms = self.elf.syms.to_vec();
        if syms.is_empty() {
            return Err(anyhow!("syms is empty"));
        }

        let file = if cfg!(debug_assertions) {
            File::create("/tmp/elf.csv").map_err(|err| anyhow!("Could not create file : {}", err))?
        } else {
            File::open("/dev/null").map_err(|err| anyhow!("Could not open null : {}", err))?
        };
        let mut writer = io::BufWriter::new(file);

        let (is_empty_key, re_key) = match Regex::new(keyword) {
            Ok(re) => (keyword.is_empty(), re),
            Err(err) => {
                eprintln!("Invalid regular expression {}: {}, donnot use", keyword, err);
                (true, Regex::new("")?)
            }
        };

        let map_iter = syms
            .iter()
            .filter_map(|sym| self.filter_symbol(sym, &self.elf.strtab, is_empty_key, &re_key, &mut writer));

        let entry_vec: Vec<SymEntry> = map_iter.collect();
        println!("[{:?}] Time of `filter_symbol`", start.elapsed());

        match entry_vec.len() {
            0 => Err(anyhow!("Cannot find")),
            1 => {
                let entry = entry_vec.first().unwrap().clone();
                println!("Matched var: {}", entry.origin_name);
                #[cfg(debug_assertions)]
                self.dw_matcher
                    .infer_var_type(&entry.origin_name, entry.mangled_name, entry.is_local_bind())
                    .ok();
                Ok(entry)
            }
            2.. => {
                println!("Matched count: {}", entry_vec.len());
                println!("Index: {:50} | var_size(B)", "var_name");
                for (i, entry) in entry_vec.iter().enumerate() {
                    println!("{:5}: {:50} | {}", i, entry.origin_name, entry.obj_size);
                }
                loop_inquire_index(&entry_vec)
            }
        }
    }

    fn filter_symbol<'c, 'b: 'c, W: io::Write>(
        &'b self,
        sym: &sym::Sym,
        strtab: &Strtab<'c>,
        is_empty_key: bool,
        re_key: &Regex,
        _bm_wrt: &mut W,
    ) -> Option<SymEntry<'c>> {
        // filter: LOCAL&OBJECT or GLOBAL&OBJECT
        if sym.st_type() != sym::STT_OBJECT
            || (sym.st_bind() != sym::STB_LOCAL && sym.st_bind() != sym::STB_GLOBAL)
        {
            return None;
        }

        let mangled_linkage = strtab.get_at(sym.st_name).unwrap_or("BAD NAME");
        let name = Name::from(mangled_linkage);
        let dem_name = name.try_demangle(DEM_OPT);
        let shn = shndx_to_str(sym.st_shndx, &self.elf.section_headers, &self.elf.shdr_strtab);

        // Must in these section: .bss .rodata .data .data.rel.ro
        if (!shn.starts_with(".bss") && !shn.starts_with(".rodata") && !shn.starts_with(".data"))
            || sym.st_size == 0
        {
            return None;
        }

        #[cfg(debug_assertions)]
        _bm_wrt
            .write_all(
                format!(
                    "{} | {} | {} | {} | {}\n",
                    sym::bind_to_str(sym.st_bind()).chars().next().unwrap_or_default(),
                    sym.st_size,
                    shn,
                    dem_name,
                    mangled_linkage
                )
                .as_bytes(),
            )
            .map_err(|err| eprintln!("Write all to file Error : {}", err))
            .unwrap_or_default();

        if RE_VAR.is_match(&dem_name) {
            return None;
        }

        if is_empty_key || re_key.is_match(&dem_name) {
            return Some(SymEntry {
                obj_addr: sym.st_value,
                obj_size: sym.st_size,
                bind_type: sym.st_bind(),
                origin_name: dem_name.to_string(),
                mangled_name: if name.detect_language() == Language::Unknown {
                    None
                } else {
                    Some(mangled_linkage)
                },
                section: shn,
            });
        }
        None
    }
}

/// the slice's len better greater than 0
pub fn loop_inquire_index<T>(entry_slice: &[T]) -> Result<T, Error>
where
    T: Clone,
{
    if entry_slice.is_empty() {
        return Err(anyhow!("The slice is empty"));
    }
    println!("Please input index to choose(default is 0): ");
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

fn shndx_to_str<'a>(idx: usize, shdrs: &'a SectionHeaders, strtab: &'a Strtab) -> Cow<'a, str> {
    if idx == 0 {
        Cow::Borrowed("")
    } else if let Some(shdr) = shdrs.get(idx) {
        if let Some(link_name) = strtab.get_at(shdr.sh_name) {
            Cow::Borrowed(link_name)
        } else {
            Cow::Owned(format!("BAD_SH_NAME_IDX={}", shdr.sh_name))
        }
    } else if idx == 0xfff1 {
        // Associated symbol is absolute.
        Cow::Borrowed("ABS")
    } else {
        Cow::Owned(format!("BAD_IDX={}", idx))
    }
}

#[cfg(test)]
mod tests {
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
        // empty &str
        let opt_re = Regex::new("");
        assert!(opt_re.is_ok());
    }

    #[test]
    fn demangle_and_detect_language() {
        // format tuple: (&str: mangled name, &str: demangled name(expect), enum[repr(u32)]: Language)
        let tuple_arr = [
            (
                "_ZN7simdutf12_GLOBAL__N_16tables13utf8_to_utf16L12utf8bigindexE",
                "simdutf::(anonymous namespace)::tables::utf8_to_utf16::utf8bigindex",
                Language::Cpp,
            ),
            (
                "_ZN7MaiData12statMemArrayE",
                "MaiData::statMemArray",
                Language::Rust, // Rust: rust or Cpp
            ),
            (
                "simple_arr",
                "simple_arr",
                Language::Unknown, // Unknown sometime includes C
            ),
            (
                "_ZL10sc_sig_arr",
                "sc_sig_arr",
                Language::Cpp, // static var-type in Cpp
            ),
            (
                "_Z11splitStringRKNSt7__cxx1112basic_stringIcSt11char_traitsIcESaIcEEEc.cold",
                "splitString(std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> > const&, char) [clone .cold]",
                Language::Cpp
            )
        ];

        for tup in tuple_arr {
            let name = Name::from(tup.0);
            assert_eq!(name.try_demangle(DEM_OPT), tup.1);
            assert_eq!(name.detect_language(), tup.2);
        }
    }
}
