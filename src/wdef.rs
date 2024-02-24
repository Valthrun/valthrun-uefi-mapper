//! Windows kernel and loader definitions

#[repr(C)]
#[allow(non_snake_case)]
pub struct ListEntry {
    pub Flink: *mut ListEntry,
    pub Blink: *mut ListEntry,
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct LoaderParameterBlock {
    pub OsMajorVersion: u32,
    pub OsMinorVersion: u32,
    pub Size: u32,
    pub OsLoaderSecurityVersion: u32,
    pub LoadOrderListHead: ListEntry,
}

#[repr(C)]
#[allow(non_snake_case, non_camel_case_types)]
pub struct KLDR_DATA_TABLE_ENTRY {
    pub InLoadOrderLinks: ListEntry,
    pub ExceptionTable: *const (),
    pub ExceptionTableSize: u32,
    pub GpValue: *const (),
    pub NonPagedDebugInfo: *const (),
    pub ImageBase: *const (),
    pub EntryPoint: *const (),
    pub SizeOfImage: u32,
    pub FullImageName: UNICODE_STRING,
    pub BaseImageName: UNICODE_STRING,
}

#[repr(C)]
#[allow(non_snake_case, non_camel_case_types)]
pub struct UNICODE_STRING {
    pub Length: u16,
    pub MaximumLength: u16,
    pub Buffer: *mut u16,
}
