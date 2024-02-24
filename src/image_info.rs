use alloc::string::ToString;

use anyhow::{
    anyhow,
    Context,
};
use obfstr::obfstr;
use uefi::{
    proto::loaded_image::LoadedImage,
    Handle,
};

use crate::{
    context::system_table,
    signature::{
        Signature,
        SignatureType,
    },
};

pub struct ImageInfo {
    pub image_base: *mut u8,
    pub image_size: usize,
}

impl ImageInfo {
    pub fn from_handle(handle: Handle) -> anyhow::Result<Self> {
        system_table()
            .boot_services()
            .open_protocol_exclusive::<LoadedImage>(handle)
            .map(|image| ImageInfo::from(&*image))
            .map_err(|err| {
                anyhow!(
                    "{}: {}",
                    obfstr!("bootmgr image is missing the LoadedImage protocol"),
                    err
                )
            })
    }

    pub fn image_as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.image_base, self.image_size) }
    }

    #[allow(unused)]
    pub fn image_as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.image_base, self.image_size) }
    }

    pub fn resolve_signature(&self, signature: &Signature) -> anyhow::Result<usize> {
        log::trace!(
            "Resolving '{}' in {:X}",
            signature.debug_name,
            self.image_base as usize
        );

        let inst_offset = signature
            .pattern
            .find(self.image_as_slice())
            .with_context(|| obfstr!("failed to find pattern").to_string())?;

        if matches!(&signature.value_type, SignatureType::Pattern) {
            let address = self.image_base.wrapping_byte_add(inst_offset) as usize;
            log::trace!("  => {:X} ({:X})", address, inst_offset);
            return Ok(address);
        }

        let value = unsafe {
            self.image_base
                .byte_add(inst_offset)
                .byte_add(signature.offset as usize)
                .cast::<u32>()
                .read_unaligned()
        };
        match &signature.value_type {
            SignatureType::Offset => {
                log::trace!("  => {:X} (inst at {:X})", value, inst_offset);
                Ok(value as usize)
            }
            SignatureType::RelativeAddress { inst_length } => {
                let value = unsafe {
                    self.image_base
                        .byte_add(inst_offset)
                        .byte_add(*inst_length)
                        .byte_offset(value as isize) as usize
                };
                log::trace!("  => {:X} ({:X})", value, value - self.image_base as usize);
                Ok(value)
            }
            SignatureType::Pattern => unreachable!(),
        }
    }
}

impl From<&LoadedImage> for ImageInfo {
    fn from(value: &LoadedImage) -> Self {
        let (image_base, image_size) = value.info();
        Self {
            image_base: image_base as *mut u8,
            image_size: image_size as usize,
        }
    }
}
