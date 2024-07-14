//! Representation of the bcachefs bkey types, derived from DWARF debug info.

use anyhow::{anyhow, Result};
use object::{Object, ObjectSection};
use std::collections::HashSet;
use std::{borrow, error, fs};

/// A list of the known bcachefs bkey types.
#[derive(Debug)]
pub struct BkeyTypes(Vec<BchStruct>);

impl BkeyTypes {
    pub fn new() -> Self {
        BkeyTypes(Vec::new())
    }

    /// Given a struct name and a member name, return the size and offset of
    /// the member within the struct, or None if it does not exist.
    pub fn get_member_layout(&self, outer: &str, member: &str) -> Option<(u64, u64)> {
        self.0
            .iter()
            .find(|i| i.name == *outer)
            .and_then(|i| i.member_layout(member))
    }
}

impl std::fmt::Display for BkeyTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for bkey in self.0.iter() {
            for memb in bkey.members.iter() {
                writeln!(
                    f,
                    "{} {} {} {}",
                    bkey.name, memb.name, memb.size, memb.offset
                )?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

/// The representation of a struct type. The only information we need
/// is the type's name and a list of its members.
#[derive(Debug)]
pub struct BchStruct {
    name: String,
    pub members: Vec<BchMember>,
}

impl BchStruct {
    /// Given a struct member name, return the size and offset of the member
    /// within its parent, or None if there is no member with the given name.
    pub fn member_layout(&self, name: &str) -> Option<(u64, u64)> {
        self.members
            .iter()
            .find(|i| i.name == *name)
            .map(|i| (i.size, i.offset))
    }
}

/// The representation of a struct member. We need its name, size, and offset
/// within the parent struct.
#[derive(Debug)]
pub struct BchMember {
    name: String,
    size: u64,
    offset: u64,
}

// The section data that will be stored in `DwarfSections` and `DwarfPackageSections`.
#[derive(Default)]
struct Section<'data> {
    data: borrow::Cow<'data, [u8]>,
}

type Reader<'data> = gimli::EndianSlice<'data, gimli::RunTimeEndian>;

fn process_file(
    object: &object::File,
    struct_list: &mut BkeyTypes,
) -> Result<(), Box<dyn error::Error>> {
    let endian = if object.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };

    fn load_section<'data>(
        object: &object::File<'data>,
        name: &str,
    ) -> Result<Section<'data>, Box<dyn error::Error>> {
        Ok(match object.section_by_name(name) {
            Some(section) => Section {
                data: section.uncompressed_data()?,
            },
            None => Default::default(),
        })
    }

    let dwarf_sections = gimli::DwarfSections::load(|id| load_section(object, id.name()))?;

    let dwarf = dwarf_sections
        .borrow(|section| gimli::EndianSlice::new(borrow::Cow::as_ref(&section.data), endian));

    let mut bkey_types = HashSet::new();
    load_bkey_types(&mut bkey_types);

    let mut iter = dwarf.units();
    while let Some(header) = iter.next()? {
        let unit = dwarf.unit(header)?;
        process_unit(&dwarf, &unit, struct_list, &mut bkey_types)?;
    }

    Ok(())
}

fn load_bkey_types(bkey_types: &mut HashSet<String>) {
    let mut ptr: *const *const i8 = unsafe { bch_bindgen::c::bch2_bkey_types.as_ptr() };
    unsafe {
        while !(*ptr).is_null() {
            let mut bkey_name = String::from("bch_");
            bkey_name.push_str(std::ffi::CStr::from_ptr(*ptr).to_str().unwrap());
            bkey_types.insert(bkey_name);
            ptr = ptr.offset(1);
        }
    }

    // This key type is not included in BCH2_BKEY_TYPES.
    bkey_types.insert("bch_inode_unpacked".to_string());
}

fn process_unit(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    struct_list: &mut BkeyTypes,
    bkey_types: &mut HashSet<String>,
) -> Result<(), gimli::Error> {
    let mut tree = unit.entries_tree(None)?;

    process_tree(dwarf, unit, tree.root()?, struct_list, bkey_types)?;

    Ok(())
}

#[derive(Clone, Copy)]
enum CompType {
    Union,
    Struct,
}

/// Used to keep track of info needed for structs that contain
/// other compound types.
struct ParentInfo<'a> {
    ty: CompType,
    starting_offset: u64,
    member_prefix: &'a str,
}

fn entry_name(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    entry: &gimli::DebuggingInformationEntry<Reader>,
) -> Option<String> {
    entry.attr(gimli::DW_AT_name).ok()?.and_then(|name| {
        Some(
            dwarf
                .attr_string(unit, name.value())
                .ok()?
                .to_string_lossy()
                .into_owned(),
        )
    })
}

fn process_tree(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    node: gimli::EntriesTreeNode<Reader>,
    struct_list: &mut BkeyTypes,
    bkey_types: &mut HashSet<String>,
) -> gimli::Result<()> {
    let entry = node.entry();
    if entry.tag() == gimli::DW_TAG_structure_type {
        let name = entry_name(dwarf, unit, entry);
        let Some(name) = name else {
            return Ok(());
        };

        if bkey_types.remove(&name) {
            let mut members: Vec<BchMember> = Vec::new();
            let parent_info = ParentInfo {
                ty: CompType::Struct,
                starting_offset: 0,
                member_prefix: "",
            };
            process_compound_type(dwarf, unit, node, &mut members, &parent_info)?;
            struct_list.0.push(BchStruct { name, members });
        }
    } else {
        let mut children = node.children();
        while let Some(child) = children.next()? {
            process_tree(dwarf, unit, child, struct_list, bkey_types)?;
        }
    }
    Ok(())
}

fn process_compound_type(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    node: gimli::EntriesTreeNode<Reader>,
    members: &mut Vec<BchMember>,
    parent: &ParentInfo,
) -> gimli::Result<()> {
    let mut children = node.children();
    while let Some(child) = children.next()? {
        process_comp_member(dwarf, unit, child, members, parent)?;
    }

    Ok(())
}

/// Given a DIE, checks if that DIE has a reference to a compound type (i.e., struct or union) and
/// if so, returns the offset in the DIE tree for that type, and the kind of compound type it is.
fn get_comp_ref(
    unit: &gimli::Unit<Reader>,
    entry: &gimli::DebuggingInformationEntry<Reader>,
) -> Option<(gimli::UnitOffset, CompType)> {
    let ref_type = entry.attr(gimli::DW_AT_type).ok()??;
    let ref_offset = match ref_type.value() {
        gimli::AttributeValue::UnitRef(offset) => offset,
        _ => return None,
    };

    let mut ty_entry = unit.entries_at_offset(ref_offset).ok()?;
    ty_entry.next_entry().ok()??;
    let ty_entry = ty_entry.current()?;

    match ty_entry.tag() {
        gimli::DW_TAG_structure_type => Some((ty_entry.offset(), CompType::Struct)),
        gimli::DW_TAG_union_type => Some((ty_entry.offset(), CompType::Union)),
        _ => None,
    }
}

fn process_comp_member(
    dwarf: &gimli::Dwarf<Reader>,
    unit: &gimli::Unit<Reader>,
    node: gimli::EntriesTreeNode<Reader>,
    members: &mut Vec<BchMember>,
    parent: &ParentInfo,
) -> gimli::Result<()> {
    let entry = node.entry().clone();

    let Some(offset) = (match parent.ty {
        CompType::Union => Some(0),
        CompType::Struct => entry
            .attr(gimli::DW_AT_data_member_location)?
            .and_then(|offset| offset.value().udata_value()),
    }) else {
        return Ok(());
    };

    let name = entry_name(dwarf, unit, &entry);

    if let Some((ref_type, comp)) = get_comp_ref(unit, &entry) {
        let prefix = if let Some(ref name) = name {
            let mut prefix = name.clone();
            prefix.push('.');
            prefix
        } else {
            String::from("")
        };
        let parent = ParentInfo {
            ty: comp,
            starting_offset: offset,
            member_prefix: &prefix,
        };
        let mut tree = unit.entries_tree(Some(ref_type))?;
        process_compound_type(dwarf, unit, tree.root()?, members, &parent)?;

        return Ok(());
    };

    let Some(size) = get_size(unit, &entry) else {
        return Ok(());
    };

    let Some(name) = name else { return Ok(()) };
    let mut name_with_prefix = String::from(parent.member_prefix);
    name_with_prefix.push_str(&name);

    members.push(BchMember {
        name: name_with_prefix,
        offset: offset + parent.starting_offset,
        size,
    });

    Ok(())
}

fn get_size(
    unit: &gimli::Unit<Reader>,
    entry: &gimli::DebuggingInformationEntry<Reader>,
) -> Option<u64> {
    if let Some(size) = entry.attr(gimli::DW_AT_byte_size).ok()? {
        return size.udata_value();
    }

    let ref_type = entry.attr(gimli::DW_AT_type).ok()??;
    if let gimli::AttributeValue::UnitRef(offset) = ref_type.value() {
        let mut type_entry = unit.entries_at_offset(offset).ok()?;
        type_entry.next_entry().ok()?;
        if let Some(t) = type_entry.current() {
            return get_size(unit, t);
        }
    }

    None
}

/// Return a list of the known bkey types and information on their field layout.
pub fn get_bkey_type_info() -> Result<BkeyTypes> {
    let path = fs::read_link("/proc/self/exe").unwrap();
    let file = fs::File::open(path).unwrap();
    let mmap = unsafe { memmap2::Mmap::map(&file).unwrap() };
    let object = object::File::parse(&*mmap).unwrap();

    let mut struct_list = BkeyTypes::new();
    process_file(&object, &mut struct_list).unwrap();

    if struct_list.0.is_empty() {
        Err(anyhow!(
            "Could not find bkey debug info.\nWas the bcachefs binary compiled with debug info?"
        ))
    } else {
        Ok(struct_list)
    }
}
