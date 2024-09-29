use std::borrow::{self, Cow};
use std::collections::BTreeSet;
use std::{error, usize};

use gimli::{AttributeValue, DwarfSections, Reader};
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

// NOTE: This type is for the convenience of `println!()`
pub type TypeOffset = usize;
pub type UniteError = Box<dyn std::error::Error + Send + Sync + 'static>;

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

    pub fn infer_var_type(
        &self,
        demangle: &str,
        mangle: Option<&'a str>,
        is_local_symbol: bool,
    ) -> Result<Vec<BTreeSet<TypeOffset>>, UniteError> {
        // Create `Reader`s for all of the sections and do preliminary parsing.
        // Alternatively, we could have used `Dwarf::load` with an owned type such as `EndianRcSlice`.
        let dwarf = self
            .dwarf_sections
            .borrow(|section| borrow_section(section, self.runtime_endian));
        let mut iter = dwarf.units();
        let mut btset_vec: Vec<_> = Vec::with_capacity(8);
        while let Some(header) = iter.next()? {
            eprint!(
                "## Unit at <.debug_info 0x{:08x}, ver: {}>",
                header.offset().as_debug_info_offset().unwrap().0,
                header.version()
            );
            let unit = dwarf.unit(header)?;
            let unit_ref = unit.unit_ref(&dwarf);

            // Temp test: Output file name of this Unit
            if let Some(program) = unit_ref.line_program.clone() {
                if let Some(file) = program.header().file_names().iter().next() {
                    eprint!(" {}", unit_ref.attr_string(file.path_name())?.to_string_lossy()?);
                }
            }
            eprintln!();
            if let Ok(addr_set) = filter_die(unit_ref, demangle, mangle, is_local_symbol) {
                btset_vec.push(addr_set);
            }
        }
        Ok(btset_vec)
    }
}

/// Iterate over the Debugging Information Entries (DIEs) in the unit_ref.
fn filter_die<'a>(
    unit_ref: gimli::UnitRef<'a, CusReader<'a>>,
    demangle: &'a str,
    opt_mangle: Option<&'a str>,
    is_local_symbol: bool,
) -> Result<BTreeSet<TypeOffset>, gimli::Error> {
    // a closure (capture argument: UnitRef<..>) to get indirect string of this DW_AT_...
    let pick_type_offset = |die: &gimli::DebuggingInformationEntry<CusReader<'a>>,
                            dw_at: gimli::DwAt,
                            target: &str|
     -> Result<TypeOffset, UniteError> {
        if let Some(linkage_val) = die.attr_value(dw_at)? {
            let reloc_rd = unit_ref.attr_string(linkage_val)?;
            let at_name = reloc_rd.to_string_lossy()?;
            if at_name == target {
                // Compilation Unit version: 5
                if let Some(type_value) = die.attr_value(gimli::DW_AT_type)? {
                    match type_value {
                        AttributeValue::UnitRef(unit_off) => Ok(unit_off.0),
                        _s => Err(format!("Found {:?}, expect UnitRef()", _s).into()),
                    }
                } else {
                    // Compilation Unit version: 4
                    if let Some(spec) = die.attr_value(gimli::DW_AT_specification)? {
                        match spec {
                            AttributeValue::UnitRef(unit_off) => Ok(unit_off.0),
                            _s => Err(format!("Found {:?}, expect UnitRef()", _s).into()),
                        }
                    } else {
                        Err("DW_AT_type and DW_AT_specification both are none".into())
                    }
                }
            } else {
                Err(format!("Not matched at_name: {}", at_name).into())
            }
        } else {
            Err(format!("DwAt: {} is none", dw_at).into())
        }
    };

    let mut entries = unit_ref.entries();
    let mut type_loc_addr = BTreeSet::new();
    while let Some((_delta_depth, die)) = entries.next_dfs()? {
        match die.tag() {
            gimli::DW_TAG_variable => {
                if is_local_symbol {
                    // May be a static var, dwarf donnot save its DW_AT_linkage_name
                    if let Ok(t_offset) = pick_type_offset(die, gimli::DW_AT_name, demangle) {
                        eprintln!("\tInsert t_offset(local): {:#x}", t_offset);
                        type_loc_addr.insert(t_offset);
                    }
                } else {
                    match opt_mangle {
                        Some(mangle) => match pick_type_offset(die, gimli::DW_AT_linkage_name, mangle) {
                            Ok(t_offset) => {
                                eprintln!("\tInsert t_offset(linkage): {:#x}", t_offset);
                                type_loc_addr.insert(t_offset);
                            }
                            Err(_dyn_err) => {}
                        },
                        // TODO: need test
                        None => {
                            if let Ok(t_offset) = pick_type_offset(die, gimli::DW_AT_name, demangle) {
                                eprintln!("\tInsert t_offset(at_name): {:#x}", t_offset);
                                type_loc_addr.insert(t_offset);
                            }
                        }
                    }
                }
            }
            // TODO: following
            gimli::DW_TAG_typedef => {}
            gimli::DW_TAG_base_type => {}
            gimli::DW_TAG_array_type => {}
            gimli::DW_TAG_structure_type => {}
            gimli::DW_TAG_class_type => {}
            gimli::DW_TAG_pointer_type => {}
            gimli::DW_TAG_const_type => {}
            _ => (),
        }
    }
    Ok(type_loc_addr)
}

// #[derive(Debug, Clone)]
// struct DefAndBaseType<'d> {
//     def_type: read::Attribute<CusReader<'d>>,
//     base_type: read::Attribute<CusReader<'d>>,
// }
