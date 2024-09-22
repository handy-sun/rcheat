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

// The reader type that will be stored in `Dwarf` and `DwarfPackage`.
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

    pub fn dump_deubg_info(&self, name: &str) -> Result<(), Box<dyn error::Error>> {
        // Create `Reader`s for all of the sections and do preliminary parsing.
        // Alternatively, we could have used `Dwarf::load` with an owned type such as `EndianRcSlice`.
        let dwarf = self
            .dwarf_sections
            .borrow(|section| borrow_section(section, self.runtime_endian));
        let mut iter = dwarf.units();

        while let Some(header) = iter.next()? {
            println!(
                "Unit at <.debug_info+0x{:x}>",
                header.offset().as_debug_info_offset().unwrap().0
            );
            let unit = dwarf.unit(header)?;
            let unit_ref = unit.unit_ref(&dwarf);
            dump_unit(unit_ref, name)?;
        }
        Ok(())
    }
}

// Iterate over the Debugging Information Entries (DIEs) in the unit.
fn dump_unit(unit: gimli::UnitRef<CusReader>, name: &str) -> Result<(), gimli::Error> {
    let mut entries = unit.entries();
    while let Some((_delta_depth, entry)) = entries.next_dfs()? {
        if entry.tag() == gimli::DW_TAG_member {
            // println!("<{}><{:06x}> {}", depth, entry.offset().0, entry.tag());
            let mut attrs = entry.attrs();
            let mut member = String::with_capacity(128);
            let mut is_match = false;
            while let Some(attr) = attrs.next()? {
                member += format!("  {}: {:?}", attr.name(), attr.value()).as_str();
                if let Ok(s) = unit.attr_string(attr.value()) {
                    let dbg_str = s.to_string_lossy()?;
                    if dbg_str == name {
                        member += format!(" {}", dbg_str).as_str();
                        is_match = true;
                    }
                }
                member += "\n";
            }
            if is_match {
                println!("{member}");
            }
        }
    }
    Ok(())
}
