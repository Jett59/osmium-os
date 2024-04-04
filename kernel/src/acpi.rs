use core::{
    fmt::{self, Debug, Formatter},
    mem::size_of,
};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{
    arch_api::acpi,
    heap::{map_physical_memory, PhysicalAddressHandle},
    memory::{reinterpret_memory, Validateable},
    paging::MemoryType,
    println,
};

pub mod fadt;
pub mod madt;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct AcpiTableHeader {
    identifier: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

const MAX_TABLE_SIZE: usize = 0x100_0000; // 16 MiB

impl Validateable for AcpiTableHeader {
    fn validate(&self) -> bool {
        // I would like to check the checksum here, but unfortunately we would need the rest of the table for that.
        // Instead we just check that the length is within a reasonable range.
        self.length as usize >= size_of::<AcpiTableHeader>()
            && self.length as usize <= MAX_TABLE_SIZE
    }
}

pub struct AcpiTableHandle {
    physical_memory_handle: PhysicalAddressHandle,
    identifier: [u8; 4],
}

impl Debug for AcpiTableHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("AcpiTableHandle")
            .field("identifier", &self.identifier)
            .finish()
    }
}

#[derive(Debug)]
pub enum AcpiTableParseError {
    InvalidHeader,
    ChecksumFailure,
}

impl AcpiTableHandle {
    /// # Safety
    /// If the provided address doesn't refer to an ACPI table, there will likely be undefined behaviour since it is impossible to tell at this point.
    /// Additionally, it is UB if the table has already been mapped, since this would create possible aliasing issues.
    pub unsafe fn new(physical_address: usize) -> Result<Self, AcpiTableParseError> {
        let physical_memory_handle = map_physical_memory(
            physical_address,
            size_of::<AcpiTableHeader>(),
            MemoryType::Normal,
        );
        let header = reinterpret_memory::<AcpiTableHeader>(&physical_memory_handle)
            .ok_or(AcpiTableParseError::InvalidHeader)?;
        let length = header.length as usize;
        let identifier = header.identifier;
        // To be certain that we don't map the same memory multiple times, we have to drop our handle before creating a new one.
        drop(physical_memory_handle);
        let physical_memory_handle =
            map_physical_memory(physical_address, length, MemoryType::Normal);
        // Check the checksum.
        let sum = physical_memory_handle
            .iter()
            .fold(0u8, |acc, &x| acc.wrapping_add(x));
        if sum != 0 {
            Err(AcpiTableParseError::ChecksumFailure)
        } else {
            Ok(Self {
                physical_memory_handle,
                identifier,
            })
        }
    }

    pub fn identifier(&self) -> &[u8; 4] {
        &self.identifier
    }

    pub fn body(&self) -> &[u8] {
        &self.physical_memory_handle[size_of::<AcpiTableHeader>()..]
    }

    pub fn body_mut(&mut self) -> &mut [u8] {
        &mut self.physical_memory_handle[size_of::<AcpiTableHeader>()..]
    }
}

#[cfg(target_arch = "aarch64")]
const REQUIRED_TABLES: &[&[u8; 4]] = &[b"APIC", b"GTDT", b"FACP"];
#[cfg(target_arch = "x86_64")]
const REQUIRED_TABLES: &[&[u8; 4]] = &[b"APIC", b"HPET"];

#[derive(Debug)]
pub enum AcpiTableSearchError {
    NoRootTable,
    InvalidRootTableSignature,
    InvalidRootTableSize,
    MissingRequiredTable(String),
    ParseError(AcpiTableParseError),
}

impl From<AcpiTableParseError> for AcpiTableSearchError {
    fn from(error: AcpiTableParseError) -> Self {
        Self::ParseError(error)
    }
}

pub fn find_required_acpi_tables() -> Result<Vec<AcpiTableHandle>, AcpiTableSearchError> {
    let root_table_address =
        acpi::get_root_table_address().ok_or(AcpiTableSearchError::NoRootTable)?;
    println!("Acpi tables at address {:#x}", root_table_address);
    // # Safety
    // The returned address is guaranteed to be valid, and we really don't have any choice but to trust it.
    // Nothing else has used the address yet, so there shouldn't be any aliasing issues.
    let root_table = unsafe { AcpiTableHandle::new(root_table_address) }?;
    if root_table.identifier() != b"RSDT" && root_table.identifier() != b"XSDT" {
        return Err(AcpiTableSearchError::InvalidRootTableSignature);
    }
    let table_body = root_table.body();
    let mut tables = Vec::new();
    if root_table.identifier() == b"RSDT" {
        if table_body.len() % size_of::<u32>() != 0 {
            return Err(AcpiTableSearchError::InvalidRootTableSize);
        }
        for table_address in table_body.array_chunks::<{ size_of::<u32>() }>() {
            let table_address = u32::from_le_bytes(*table_address) as usize;
            // # Safety
            // It is obviously safe to interpret the pointers in the RSDT as ACPI tables, since that is the point of the RSDT.
            let table = unsafe { AcpiTableHandle::new(table_address)? };
            println!(
                "Found table: {}",
                String::from_utf8_lossy(table.identifier())
            );
            if REQUIRED_TABLES.contains(&table.identifier()) {
                tables.push(table);
            }
        }
    } else {
        if table_body.len() % size_of::<u64>() != 0 {
            return Err(AcpiTableSearchError::InvalidRootTableSize);
        }
        for table_address in table_body.array_chunks::<{ size_of::<u64>() }>() {
            let table_address = u64::from_le_bytes(*table_address) as usize;
            // # Safety
            // It is obviously safe to interpret the pointers in the XSDT as ACPI tables, since that is the point of the XSDT.
            let table = unsafe { AcpiTableHandle::new(table_address) }?;
            println!(
                "Found table: {}",
                String::from_utf8_lossy(table.identifier())
            );
            if REQUIRED_TABLES.contains(&table.identifier()) {
                tables.push(table);
            }
        }
    }
    // Check that we found at least one table for each of required tables.
    for required_table in REQUIRED_TABLES {
        if !tables
            .iter()
            .any(|table| table.identifier() == *required_table)
        {
            return Err(AcpiTableSearchError::MissingRequiredTable(
                #[allow(clippy::explicit_auto_deref)]
                String::from_utf8_lossy(*required_table).to_string(),
            ));
        }
    }
    Ok(tables)
}
