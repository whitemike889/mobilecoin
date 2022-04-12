// Copyright (c) 2018-2021 The MobileCoin Foundation

//! An implementation of the fog-ocall-oram-storage-edl interface
//!
//! This crate implements and exports the functions defined in the EDL file.
//! This is the only public API of this crate, everything else is an
//! implementation detail.
//!
//! Main ideas:
//! Instead of a global data structure protected by a mutex, this API does
//! the following:
//!
//! On enclave allocation request:
//! - Create an UntrustedAllocation on the heap (Box::new)
//! - This "control structure" contains the creation parameters of the
//!   allocation, and pointers to the block storage regions, created using
//!   ~malloc
//! - The allocation_id u64, is the value of this pointer The box is
//!   reconstituted whenever the enclave wants to access the allocation
//! - The box is freed when the enclave releases the allocation (This probably
//!   won't actually happen in production)
//!
//! When debug assertions are on, we keep track in a global variable which ids
//! are valid and which ones aren't so that we can give nice panic messages and
//! avoid memory corruption, if something really bad is happening in the enclave
//! and it is corrupting the id numbers.
//!
//! Note: There is some gnarly pointer-arithmetic stuff happening around the
//! copy_slice_nonoverlapping stuff. The reason this is happening is, on the
//! untrusted side, we do not know data_item_size and meta_item_size statically.
//! So while on the trusted side, it all works nicely in the type system, in
//! this side, we have to do a little arithmetic ourselves.
//! It is untenable for the untrusted side to also know these sizes statically,
//! it would create a strange coupling in the build process.

#![deny(missing_docs)]
#![feature(test)]

extern crate test;

use libc;
use mc_common::logger::global_log;
use std::{
    boxed::Box,
    ffi::CString,
    ptr,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

const MOUNTED_DEVICE_PATH: &str = "/dev/sdc";

/// Resources held on untrusted side in connection to an allocation request by
/// enclave
///
/// This is not actually part of the public interface of the crate, the only
/// thing exported by the crate is the enclave EDL functions
struct UntrustedAllocation {
    /// The number of data and meta items stored in this allocation
    count: usize,
    /// The size of a data item in bytes
    data_item_size: usize,
    /// The size of a meta item in bytes
    meta_item_size: usize,
    /// The pointer to the data items that are stored in memory mapped to
    /// a file using mmap.
    data_pointer: *mut u64,
    /// The pointer to the meta items that are stored in memory mapped to
    /// a file using mmap.
    meta_pointer: *mut u64,
    /// A flag set to true when a thread is in the critical section and released
    /// when it leaves. This is used to trigger assertions if there is a
    /// race happening on this API This is simpler and less expensive than
    /// an actual mutex to protect critical sections
    critical_section_flag: AtomicBool,
    /// A flag set to true when there is an active checkout. This is used to
    /// trigger assertions if checkout are not followed by checkin
    /// operation.
    checkout_flag: AtomicBool,
}

/// Tracks total memory allocated via this mechanism for logging purposes
static TOTAL_MEM_FOOTPRINT_KB: AtomicU64 = AtomicU64::new(0);

/// Helper which computes the total memory in kb allocated for count,
/// data_item_size, meta_item_size
fn compute_mem_kb(count: usize, data_item_size: usize, meta_item_size: usize) -> u64 {
    let num_bytes = (count * (data_item_size + meta_item_size)) as u64;
    // Divide by 1024 and round up, to compute num_bytes in kb
    (num_bytes + 1023) / 1024
}

impl UntrustedAllocation {
    /// Create a new untrusted allocation for given count and item sizes, on the
    /// heap
    ///
    /// Data and meta item sizes must be divisible by 8, consistent with the
    /// contract described in the edl file
    pub fn new(count: usize, data_item_size: usize, meta_item_size: usize) -> Self {
        let mem_kb = compute_mem_kb(count, data_item_size, meta_item_size);
        let total_mem_kb = mem_kb + TOTAL_MEM_FOOTPRINT_KB.fetch_add(mem_kb, Ordering::SeqCst);
        global_log::info!("Untrusted is allocating oram storage: count = {}, data_size = {}, meta_size = {}, mem = {} KB. Total mem allocated this way = {} KB", count, data_item_size, meta_item_size, mem_kb, total_mem_kb);
        assert!(
            data_item_size % 8 == 0,
            "data item size is not good: {}",
            data_item_size
        );
        assert!(
            meta_item_size % 8 == 0,
            "meta item size is not good: {}",
            meta_item_size
        );

        let data_pointer: *mut libc::c_void = unsafe {
            // We do not care about setting a specific address in memory.
            let memory_start_address: *mut libc::c_void = ptr::null_mut();
            let memory_length: libc::size_t = count * data_item_size;
            let file_protections: libc::c_int = libc::PROT_READ | libc::PROT_WRITE;
            // Use MAP_PRIVATE because no other process needs to access the underlying file.
            let flags: libc::c_int = libc::MAP_PRIVATE;
            let offset: libc::off_t = 0;
            let file_name = CString::new(MOUNTED_DEVICE_PATH).unwrap();
            let file_descriptor =
                libc::open(file_name.as_ptr() as *const libc::c_char, libc::O_RDWR);
            libc::mmap(
                memory_start_address,
                memory_length,
                file_protections,
                flags,
                file_descriptor,
                offset,
            )
        };
        if data_pointer.is_null() {
            panic!(
                "Could not allocate memory for data segment: {}",
                count * data_item_size
            )
        }

        let meta_pointer: *mut libc::c_void = unsafe {
            // We do not care about setting a specific address in memory.
            let memory_start_address: *mut libc::c_void = ptr::null_mut();
            let memory_length: libc::size_t = count * meta_item_size;
            let file_protections: libc::c_int = libc::PROT_READ | libc::PROT_WRITE;
            // We use:
            //  - MAP_PRIVATE because no other process needs to access the underlying file.
            //  - MAP_ANONYMOUS because we don't need to map to a particular device for meta
            //    since it doesn't require a lot of memory (unlike data).
            let flags: libc::c_int = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS;
            // We are using an anymous map, which means we are not interfacing
            // with a file.
            let file_descriptor: libc::c_int = -1;
            let offset: libc::off_t = 0;
            libc::mmap(
                memory_start_address,
                memory_length,
                file_protections,
                flags,
                file_descriptor,
                offset,
            )
        };
        if meta_pointer.is_null() {
            panic!(
                "Could not allocate memory for meta segment: {}",
                count * meta_item_size
            )
        }

        let critical_section_flag = AtomicBool::new(false);
        let checkout_flag = AtomicBool::new(false);

        Self {
            count,
            data_item_size,
            meta_item_size,
            data_pointer: data_pointer as *mut u64,
            meta_pointer: meta_pointer as *mut u64,
            critical_section_flag,
            checkout_flag,
        }
    }
}

impl Drop for UntrustedAllocation {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(
                self.data_pointer as *mut libc::c_void,
                self.count * self.data_item_size,
            );
            libc::munmap(
                self.meta_pointer as *mut libc::c_void,
                self.count * self.meta_item_size,
            );
            let mem_kb = compute_mem_kb(self.count, self.data_item_size, self.meta_item_size);
            TOTAL_MEM_FOOTPRINT_KB.fetch_sub(mem_kb, Ordering::SeqCst);
        }
    }
}

// These extern "C" functions must match edl file

/// # Safety
///
/// meta_size and data_size must be divisible by 8
/// id_out must be a valid pointer to a u64
#[no_mangle]
pub unsafe extern "C" fn allocate_oram_storage(
    count: u64,
    data_size: u64,
    meta_size: u64,
    id_out: *mut u64,
) {
    let result = Box::new(UntrustedAllocation::new(
        count as usize,
        data_size as usize,
        meta_size as usize,
    ));
    let id = Box::into_raw(result) as u64;
    #[cfg(debug_assertions)]
    debug_checks::add_id(id);
    *id_out = id;
}

/// # Safety
///
/// id must be a valid id previously returned by allocate_oram_storage
#[no_mangle]
pub unsafe extern "C" fn release_oram_storage(id: u64) {
    let ptr: *mut UntrustedAllocation = core::mem::transmute(id);
    assert!(
        !(*ptr).critical_section_flag.swap(true, Ordering::SeqCst),
        "Could not enter critical section when releasing storage"
    );
    let _get_dropped = Box::<UntrustedAllocation>::from_raw(ptr);
    #[cfg(debug_assertions)]
    debug_checks::remove_id(id);
}

/// # Safety
///
/// idx must point to a buffer of length idx_len
/// databuf must point to a buffer of length databuf_len
/// metabuf must point to a buffer of length metabuf_len
///
/// id must be a valid id previously returned by allocate_oram_storage
///
/// databuf_len must be equal to idx_len * data_item_size / 8,
/// where data_item_size was passed when allocating storage.
///
/// metabuf_len must be equal to idx_len * meta_item_size / 8,
/// where meta_item_size was passed when allocating storage.
///
/// All indices must be in bounds, less than count that was passed when
/// allocaitng.
#[no_mangle]
pub unsafe extern "C" fn checkout_oram_storage(
    id: u64,
    idx: *const u64,
    idx_len: usize,
    databuf: *mut u64,
    databuf_len: usize,
    metabuf: *mut u64,
    metabuf_len: usize,
) {
    global_log::trace!("checkout_oram_storage: id = {}, idx = {:?}, idx_len = {}, databuf = {:?}, databuf_len = {}, metabuf = {:?}, metabuf_len = {}", id, idx, idx_len, databuf, databuf_len, metabuf, metabuf_len);
    #[cfg(debug_assertions)]
    debug_checks::check_id(id);
    let ptr: *const UntrustedAllocation = core::mem::transmute(id);
    assert!(
        !(*ptr).critical_section_flag.swap(true, Ordering::SeqCst),
        "Could not enter critical section when checking out storage"
    );
    assert!(
        !(*ptr).checkout_flag.swap(true, Ordering::SeqCst),
        "illegal checkout"
    );

    // The size of a data_item, measured in u64's
    let data_copy_size = (*ptr).data_item_size / core::mem::size_of::<u64>();
    // The size of a meta_item, measured in u64's
    let meta_copy_size = (*ptr).meta_item_size / core::mem::size_of::<u64>();

    assert!(idx_len * data_copy_size == databuf_len);
    assert!(idx_len * meta_copy_size == metabuf_len);

    let indices = core::slice::from_raw_parts(idx, idx_len);

    for (count, index) in indices.iter().enumerate() {
        let index = *index as usize;
        core::ptr::copy_nonoverlapping(
            (*ptr).data_pointer.add(data_copy_size * index),
            databuf.add(data_copy_size * count),
            data_copy_size,
        );
    }

    for (count, index) in indices.iter().enumerate() {
        let index = *index as usize;
        core::ptr::copy_nonoverlapping(
            (*ptr).meta_pointer.add(meta_copy_size * index),
            metabuf.add(meta_copy_size * count),
            meta_copy_size,
        );
    }

    assert!(
        (*ptr).critical_section_flag.swap(false, Ordering::SeqCst),
        "Could not leave critical section when checking out storage"
    );
}

/// # Safety
///
/// idx must point to a buffer of length idx_len
/// databuf must point to a buffer of length databuf_len
/// metabuf must point to a buffer of length metabuf_len
///
/// id must be a valid id previously returned by allocate_oram_storage
///
/// databuf_len must be equal to idx_len * data_item_size / 8,
/// where data_item_size was passed when allocating storage.
///
/// metabuf_len must be equal to idx_len * meta_item_size / 8,
/// where meta_item_size was passed when allocating storage.
///
/// All indices must be in bounds, less than count that was passed when
/// allocaitng.
#[no_mangle]
pub unsafe extern "C" fn checkin_oram_storage(
    id: u64,
    idx: *const u64,
    idx_len: usize,
    databuf: *const u64,
    databuf_len: usize,
    metabuf: *const u64,
    metabuf_len: usize,
) {
    global_log::trace!("checkin_oram_storage: id = {}, idx = {:?}, idx_len = {}, databuf = {:?}, databuf_len = {}, metabuf = {:?}, metabuf_len = {}", id, idx, idx_len, databuf, databuf_len, metabuf, metabuf_len);
    #[cfg(debug_assertions)]
    debug_checks::check_id(id);
    let ptr: *const UntrustedAllocation = core::mem::transmute(id);
    assert!(
        !(*ptr).critical_section_flag.swap(true, Ordering::SeqCst),
        "Could not enter critical section when checking in storage"
    );
    assert!(
        (*ptr).checkout_flag.swap(false, Ordering::SeqCst),
        "illegal checkin"
    );

    // The size of a data_item, measured in u64's
    let data_copy_size = (*ptr).data_item_size / core::mem::size_of::<u64>();
    // The size of a meta_item, measured in u64's
    let meta_copy_size = (*ptr).meta_item_size / core::mem::size_of::<u64>();

    assert!(idx_len * data_copy_size == databuf_len);
    assert!(idx_len * meta_copy_size == metabuf_len);

    let indices = core::slice::from_raw_parts(idx, idx_len);

    for (count, index) in indices.iter().enumerate() {
        let index = *index as usize;
        core::ptr::copy_nonoverlapping(
            databuf.add(data_copy_size * count),
            (*ptr).data_pointer.add(data_copy_size * index),
            data_copy_size,
        );
    }

    for (count, index) in indices.iter().enumerate() {
        let index = *index as usize;
        core::ptr::copy_nonoverlapping(
            metabuf.add(meta_copy_size * count),
            (*ptr).meta_pointer.add(meta_copy_size * index),
            meta_copy_size,
        );
    }

    assert!(
        (*ptr).critical_section_flag.swap(false, Ordering::SeqCst),
        "Could not leave critical section when checking in storage"
    );
}

// This module is only used in debug builds, it allows us to ensure that an id
// is valid before we cast it to a pointer, and give nicer asserts if it isn't
#[cfg(debug_assertions)]
mod debug_checks {
    use std::{collections::BTreeSet, sync::Mutex};

    pub fn add_id(id: u64) {
        let mut lk = VALID_IDS.lock().unwrap();
        assert!(!lk.contains(&id), "id already exists");
        lk.insert(id);
    }
    pub fn remove_id(id: u64) {
        let mut lk = VALID_IDS.lock().unwrap();
        assert!(lk.contains(&id), "can't remove non-existant id");
        lk.remove(&id);
    }
    pub fn check_id(id: u64) {
        let lk = VALID_IDS.lock().unwrap();
        assert!(lk.contains(&id), "invalid id passed from enclave");
    }

    lazy_static::lazy_static! {
        static ref VALID_IDS: Mutex<BTreeSet<u64>> = Mutex::new(Default::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[bench]
    fn bench_allocate_oram_storage(b: &mut Bencher) {
        //let count = 57344;
        let count = 1000;
        let data_size = 4096;
        let meta_size = 104;
        let mut id: u64 = 0;
        let id_out: *mut u64 = &mut id;

        unsafe {
            b.iter(|| allocate_oram_storage(count, data_size, meta_size, id_out));
            release_oram_storage(*id_out);
        }
    }

    #[bench]
    fn bench_checkout_checkin_oram_storage(b: &mut Bencher) {
        //let count = 57344;
        let count = 1000;
        let data_size = 4096;
        let meta_size = 104;
        let mut id: u64 = 0;
        let id_out: *mut u64 = &mut id;

        unsafe {
            allocate_oram_storage(count, data_size, meta_size, id_out);
            let checkout_idx: [u64; 10] = [0; 10];
            let mut checkout_databuf: [u64; 5120] = [0; 5120];
            let mut checkout_metabuf: [u64; 130] = [0; 130];

            let checkin_idx: [u64; 10] = [0; 10];
            let checkin_databuf: [u64; 5120] = [2; 5120];
            let checkin_metabuf: [u64; 130] = [3; 130];

            b.iter(|| {
                checkout_oram_storage(
                    *id_out,
                    checkout_idx.as_ptr(),
                    checkout_idx.len(),
                    checkout_databuf.as_mut_ptr(),
                    checkout_databuf.len(),
                    checkout_metabuf.as_mut_ptr(),
                    checkout_metabuf.len(),
                );
                checkin_oram_storage(
                    *id_out,
                    &checkin_idx as *const u64,
                    checkin_idx.len(),
                    &checkin_databuf as *const u64,
                    checkin_databuf.len(),
                    &checkin_metabuf as *const u64,
                    checkin_metabuf.len(),
                );
            });

            release_oram_storage(*id_out);
            println!("Released oram storage: {}", *id_out);
        }
    }

    // #[bench]
    // fn bench_checkout_oram_storage(b: &mut Bencher) {
    // }
}
