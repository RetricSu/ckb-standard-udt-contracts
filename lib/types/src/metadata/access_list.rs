use alloc::vec::Vec;

use crate::molecule::prelude::{Builder, Entity};

use crate::{error::Error, generated};

use super::config::MAX_ACCESSLIST_ENTRIES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessListRange {
    pub start: [u8; 32],
    pub end: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessListShard {
    pub range: AccessListRange,
    pub entries: Vec<[u8; 32]>,
}

impl AccessListShard {
    pub fn from_slice(data: &[u8]) -> Result<Self, Error> {
        Self::try_from(data)
    }

    pub fn to_molecule(&self) -> Result<generated::metadata::AccessListShard, Error> {
        if self.entries.len() > MAX_ACCESSLIST_ENTRIES {
            return Err(Error::AccessListTooLarge);
        }
        validate_shard(self)?;

        let mut entries = generated::blockchain::Byte32Vec::new_builder();
        for entry in &self.entries {
            entries = entries.push(generated::blockchain::Byte32::from_slice(entry)?);
        }

        Ok(generated::metadata::AccessListShard::new_builder()
            .range(
                generated::metadata::AccessListRange::new_builder()
                    .start(generated::blockchain::Byte32::from_slice(
                        &self.range.start,
                    )?)
                    .end(generated::blockchain::Byte32::from_slice(&self.range.end)?)
                    .build(),
            )
            .entries(entries.build())
            .build())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        Ok(self.to_molecule()?.as_slice().to_vec())
    }
}

impl TryFrom<&[u8]> for AccessListShard {
    type Error = Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let raw = generated::metadata::AccessListShard::from_slice(data)?;
        if raw.entries().len() > MAX_ACCESSLIST_ENTRIES {
            return Err(Error::AccessListTooLarge);
        }

        let shard = Self {
            range: AccessListRange {
                start: raw.range().start().into(),
                end: raw.range().end().into(),
            },
            entries: raw.entries().into_iter().map(Into::into).collect(),
        };
        validate_shard(&shard)?;
        Ok(shard)
    }
}

fn validate_shard(shard: &AccessListShard) -> Result<(), Error> {
    if shard.range.start > shard.range.end
        || !is_nibble_aligned_range(&shard.range.start, &shard.range.end)
    {
        return Err(Error::AccessListInvalidRange);
    }

    let mut previous = None;
    for entry in &shard.entries {
        if entry < &shard.range.start || entry > &shard.range.end {
            return Err(Error::AccessListEntryOutOfRange);
        }
        if let Some(previous_entry) = previous {
            if entry == previous_entry {
                return Err(Error::AccessListEntriesDuplicated);
            }
            if entry < previous_entry {
                return Err(Error::AccessListEntriesNotSorted);
            }
        }
        previous = Some(entry);
    }

    Ok(())
}

fn is_nibble_aligned_range(start: &[u8; 32], end: &[u8; 32]) -> bool {
    is_nibble_aligned_start(start) && is_nibble_aligned_end(end)
}

fn is_nibble_aligned_start(start: &[u8; 32]) -> bool {
    start[0] & 0x0f == 0x00 && start[1..].iter().all(|byte| *byte == 0x00)
}

fn is_nibble_aligned_end(end: &[u8; 32]) -> bool {
    end[0] & 0x0f == 0x0f && end[1..].iter().all(|byte| *byte == 0xff)
}
