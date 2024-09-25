use std::borrow::{self, Cow};
use std::error;

use gimli::{DwarfSections, Reader};
use object::{Object, ObjectSection};
// use regex::bytes::Regex;

// This is a simple wrapper around `object::read::RelocationMap` that implements
// `gimli::read::Relocate` for use with `gimli::RelocateReader`.
#[derive(Debug, Default)]
struct RelocMap(object::read::RelocationMap);

// The section data that will be stored in `DwarfSections` and `DwarfPackageSections`.
#[derive(Default)]
struct CusSection<'d> {
    data: Cow<'d, [u8]>,
    relocations: RelocMap,
}

impl<'a> gimli::read::Relocate for &'a RelocMap {
    fn relocate_address(&self, offset: usize, value: u64) -> gimli::Result<u64> {
        Ok(self.0.relocate(offset as u64, value))
    }

    fn relocate_offset(&self, offset: usize, value: usize) -> gimli::Result<usize> {
        <usize as gimli::ReaderOffset>::from_u64(self.0.relocate(offset as u64, value as u64))
    }
}

// The custom reader type that will be stored in `Dwarf` and `DwarfPackage`.
// If you don't need relocations, you can use `gimli::EndianSlice` directly.
type CusReader<'d> = gimli::RelocateReader<gimli::EndianSlice<'d, gimli::RunTimeEndian>, &'d RelocMap>;

// Borrow a `Section` to create a `Reader`.
fn borrow_section<'d>(section: &'d CusSection<'d>, endian: gimli::RunTimeEndian) -> CusReader<'d> {
    let slice = gimli::EndianSlice::new(borrow::Cow::as_ref(&section.data), endian);
    gimli::RelocateReader::new(slice, &section.relocations)
}

fn load_section<'d>(object: &object::File<'d>, name: &str) -> Result<CusSection<'d>, Box<dyn error::Error>> {
    Ok(match object.section_by_name(name) {
        Some(section) => CusSection {
            data: section.uncompressed_data()?,
            relocations: section.relocation_map().map(RelocMap)?,
        },
        None => Default::default(),
    })
}

pub struct DwarfInfoMatcher<'a> {
    dwarf_sections: DwarfSections<CusSection<'a>>,
    runtime_endian: gimli::RunTimeEndian,
}

impl<'a> DwarfInfoMatcher<'a> {
    pub fn parse(byte_slice: &'a [u8]) -> Result<Self, Box<dyn error::Error>> {
        let obj_file = object::File::parse(byte_slice)?;
        Ok(Self {
            dwarf_sections: gimli::DwarfSections::load(|id| load_section(&obj_file, id.name()))?,
            runtime_endian: if obj_file.is_little_endian() {
                gimli::RunTimeEndian::Little
            } else {
                gimli::RunTimeEndian::Big
            },
        })
    }

    pub fn infer_var_type(&self, demangle: &str, mangle: &str) -> Result<String, Box<dyn error::Error>> {
        // Create `Reader`s for all of the sections and do preliminary parsing.
        // Alternatively, we could have used `Dwarf::load` with an owned type such as `EndianRcSlice`.
        let dwarf = self
            .dwarf_sections
            .borrow(|section| borrow_section(section, self.runtime_endian));
        let mut iter = dwarf.units();

        while let Some(header) = iter.next()? {
            println!(
                "Unit at <.debug_info {:#x}>",
                header.offset().as_debug_info_offset().unwrap().0
            );
            let unit = dwarf.unit(header)?;
            let unit_ref = unit.unit_ref(&dwarf);
            if let Ok(at_name) = filter_die_in_unit(unit_ref, demangle, mangle) {
                if !at_name.is_empty() {
                    return Ok(at_name.into_owned());
                }
            }
        }
        Err("Not find in debug".into())
    }
}

// Iterate over the Debugging Information Entries (DIEs) in the unit.
fn filter_die_in_unit<'a>(
    unit: gimli::UnitRef<'a, CusReader<'a>>,
    demangle: &'a str,
    mangle: &'a str,
) -> Result<Cow<'a, str>, gimli::Error> {
    let mut entries = unit.entries();
    while let Some((_delta_depth, entry)) = entries.next_dfs()? {
        match entry.tag() {
            gimli::DW_TAG_variable => {
                if let Some(attr_val) = entry.attr_value(gimli::DW_AT_name)? {
                    let reloc_reader_at = unit.attr_string(attr_val)?;
                    let real_at_name = reloc_reader_at.to_string_lossy()?;
                    if demangle.contains(&*real_at_name) {
                        if mangle.is_empty() {
                            return Ok(Cow::Owned(real_at_name.into_owned()));
                        }

                        if let Some(linkage_val) = entry.attr_value(gimli::DW_AT_linkage_name)? {
                            let reloc_reader_at_link = unit.attr_string(linkage_val)?;
                            let linkage_name = reloc_reader_at_link.to_string_lossy()?;
                            if linkage_name == mangle {
                                return Ok(Cow::Owned(real_at_name.into_owned()));
                            }
                        }
                    } else {
                        println!("var_at: {}", real_at_name);
                    }
                }
            }
            gimli::DW_TAG_typedef => {}
            gimli::DW_TAG_base_type => {}
            gimli::DW_TAG_array_type => {}
            gimli::DW_TAG_structure_type => {}
            gimli::DW_TAG_pointer_type => {}
            _ => (),
        }
    }
    Ok(Cow::Borrowed(""))
}

// #[derive(Debug, Clone)]
// struct DefAndBaseType<'d> {
//     def_type: read::Attribute<CusReader<'d>>,
//     base_type: read::Attribute<CusReader<'d>>,
// }
