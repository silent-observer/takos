use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use log::info;

use bitflags::bitflags;

use super::blockdevice::RandomAccessDevice;

const SECTOR_SIZE: usize = 512;
pub struct Fat32Filesystem {
    device: Box<dyn RandomAccessDevice + Send + Sync>,
    sectors_per_cluster: usize,
    sectors_per_fat: usize,
    fat_offset: usize,
    cluster_start_offset: usize,
    root_dir_first_cluster: u32,
}

bitflags! {
    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct FatFileAttributes: u8 {
        const READ_ONLY = 0x01;
        const HIDDEN = 0x02;
        const SYSTEM = 0x04;
        const VOLUME_ID = 0x08;
        const DIRECTORY = 0x10;
        const ARCHIVE = 0x20;
        const LONG_NAME = 0x0F;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct FatDirectoryEntry {
    filename: [u8; 8],
    extension: [u8; 3],
    attributes: FatFileAttributes,
    reserved1: [u8; 8],
    cluster_high: u16,
    reserved2: [u8; 4],
    cluster_low: u16,
    file_size: u32,
}

impl FatDirectoryEntry {
    fn new_from(bytes: &[u8; 32]) -> Self {
        let mut result = unsafe { &*(bytes as *const [u8; 32] as *const Self) }.clone();
        result.cluster_high = u16::from_le(result.cluster_high);
        result.cluster_low = u16::from_le(result.cluster_low);
        result.file_size = u32::from_le(result.file_size);
        result
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryEntryData {
    filename: String,
    attributes: FatFileAttributes,
    file_size: u32,
    cluster: u32,
}

#[derive(Debug, Clone)]
pub enum FatError {
    NonAbsolutePath,
    PathDoesntExist(String),
    NotADirectory(String),
}

impl Fat32Filesystem {
    pub fn new(device: impl RandomAccessDevice + Send + Sync + 'static) -> Self {
        let bytes_per_sector = device.read(0x0B, 2);
        assert_eq!(bytes_per_sector, &[0x00, 0x02]);
        let sectors_per_cluster = device.read(0x0D, 1)[0] as usize;
        let reserved_clusters = device.read(0x0E, 2);
        assert_eq!(reserved_clusters, &[0x20, 0x00]);
        let number_of_fats = device.read(0x10, 1)[0];
        assert_eq!(number_of_fats, 2);
        let sectors_per_fat = u32::from_le_bytes(device.read(0x24, 4).try_into().unwrap()) as usize;
        let root_directory_first_cluster = device.read(0x2C, 4);
        assert_eq!(root_directory_first_cluster, &[0x02, 0x00, 0x00, 0x00]);
        let signature = device.read(0x1FE, 2);
        assert_eq!(signature, &[0x55, 0xAA]);

        Self {
            sectors_per_cluster,
            sectors_per_fat,
            fat_offset: 32,
            cluster_start_offset: 32 + (number_of_fats as usize * sectors_per_fat),
            root_dir_first_cluster: 2,
            device: Box::new(device),
        }
    }

    fn cluster_address(&self, cluster: u32) -> usize {
        (self.cluster_start_offset + ((cluster as usize - 2) * self.sectors_per_cluster))
            * SECTOR_SIZE
    }

    fn next_cluster(&self, cluster: u32) -> Option<u32> {
        let fat_entry_addr = self.fat_offset * SECTOR_SIZE + cluster as usize * 4;
        let fat_entry: [u8; 4] = self.device.read(fat_entry_addr, 4).try_into().unwrap();
        let fat_entry = u32::from_le_bytes(fat_entry);
        info!("Fat entry: 0x{:X}", fat_entry);
        if fat_entry == 0 || fat_entry >= 0x0FFF_FFFF {
            None
        } else {
            Some(fat_entry)
        }
    }

    fn read_chain(&self, start_cluster: u32) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut current_cluster = start_cluster;
        loop {
            let cluster_address = self.cluster_address(current_cluster);
            chain.push(cluster_address);
            if let Some(next_cluster) = self.next_cluster(current_cluster) {
                current_cluster = next_cluster;
            } else {
                break;
            }
        }
        chain
    }

    fn read_chain_full(&self, start_cluster: u32) -> Vec<u8> {
        let chain = self.read_chain(start_cluster);
        let mut result = Vec::with_capacity(chain.len() * SECTOR_SIZE * self.sectors_per_cluster);
        for addr in chain {
            let data = self
                .device
                .read(addr, SECTOR_SIZE * self.sectors_per_cluster);
            result.extend_from_slice(data);
        }
        result
    }

    fn directory_entry_iter(&self, start_cluster: u32) -> FatDirectoryEntryIterator {
        let directory_data = self.read_chain_full(start_cluster);
        FatDirectoryEntryIterator {
            directory_data,
            index: 0,
        }
    }

    fn find_file(&self, current_dir: u32, filename: &str) -> Result<DirectoryEntryData, FatError> {
        let filename_uppercase = filename.to_uppercase();
        for entry in self.directory_entry_iter(current_dir) {
            if entry.filename.to_uppercase() == filename_uppercase {
                return Ok(entry);
            }
        }
        Err(FatError::PathDoesntExist(filename.to_string()))
    }

    fn find_file_full(&self, path: &str) -> Result<DirectoryEntryData, FatError> {
        let mut current_entry = DirectoryEntryData {
            filename: String::from("/"),
            attributes: FatFileAttributes::DIRECTORY,
            file_size: 0,
            cluster: self.root_dir_first_cluster,
        };
        if !path.starts_with('/') {
            return Err(FatError::NonAbsolutePath);
        }
        for (i, part) in path[1..].split('/').enumerate() {
            if part.is_empty() {
                continue;
            }
            if !current_entry
                .attributes
                .contains(FatFileAttributes::DIRECTORY)
            {
                return Err(FatError::NotADirectory(
                    part.split('/')
                        .take(i + 1)
                        .fold(String::new(), |x, y| x + "/" + y),
                ));
            }
            let entry = self.find_file(current_entry.cluster, part)?;
            current_entry = entry;
        }
        Ok(current_entry)
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, FatError> {
        let entry = self.find_file_full(path)?;
        let mut data = self.read_chain_full(entry.cluster);
        data.truncate(entry.file_size as usize);
        Ok(data)
    }

    pub fn dir_iter(&self, path: &str) -> Result<DirectoryIterator, FatError> {
        let entry = self.find_file_full(path)?;
        if !entry.attributes.contains(FatFileAttributes::DIRECTORY) {
            Err(FatError::NotADirectory(path.to_string()))
        } else {
            Ok(DirectoryIterator(self.directory_entry_iter(entry.cluster)))
        }
    }
}

struct FatDirectoryEntryIterator {
    directory_data: Vec<u8>,
    index: usize,
}

impl Iterator for FatDirectoryEntryIterator {
    type Item = DirectoryEntryData;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let offset = self.index * 0x20;
            return match self.directory_data[offset] {
                0x00 => None,
                0xE5 => {
                    self.index += 1;
                    continue;
                }
                _ => {
                    let attr_offset = offset + 0x0B;
                    let attr =
                        FatFileAttributes::from_bits(self.directory_data[attr_offset]).unwrap();
                    if attr == FatFileAttributes::LONG_NAME {
                        self.index += 1;
                        continue;
                    } else {
                        let entry = FatDirectoryEntry::new_from(
                            &self.directory_data[offset..offset + 32].try_into().unwrap(),
                        );
                        let filename = String::from_utf8(entry.filename.to_vec())
                            .unwrap()
                            .trim_end()
                            .to_string();
                        let extension = String::from_utf8(entry.extension.to_vec())
                            .unwrap()
                            .trim_end()
                            .to_string();
                        let filename = if extension.is_empty() {
                            filename
                        } else {
                            format!("{}.{}", filename, extension)
                        };
                        self.index += 1;
                        Some(DirectoryEntryData {
                            filename,
                            attributes: entry.attributes,
                            file_size: entry.file_size,
                            cluster: (entry.cluster_high as u32) << 16 | entry.cluster_low as u32,
                        })
                    }
                }
            };
        }
    }
}

pub struct DirectoryIterator(FatDirectoryEntryIterator);

impl Iterator for DirectoryIterator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.0.next()?.filename)
    }
}
