// Copyright (c) 2025 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache Software License 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
// which is available at https://opensource.org/licenses/MIT.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

pub use iceoryx2_pal_concurrency_sync::atomic::Ordering;

use iceoryx2_bb_elementary_traits::{
    placement_default::PlacementDefault, zero_copy_send::ZeroCopySend, zeroable::Zeroable,
};

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
core::arch::global_asm!(
    ".weak __tsan_acquire",
    ".weak __tsan_release",
    ".hidden iox2_tsan_acquire_hook",
    ".globl iox2_tsan_acquire_hook",
    ".type iox2_tsan_acquire_hook, @function",
    "iox2_tsan_acquire_hook:",
    "mov rax, qword ptr [rip + __tsan_acquire@GOTPCREL]",
    "test rax, rax",
    "jz 2f",
    "jmp rax",
    "2:",
    "ret",
    ".hidden iox2_tsan_release_hook",
    ".globl iox2_tsan_release_hook",
    ".type iox2_tsan_release_hook, @function",
    "iox2_tsan_release_hook:",
    "mov rax, qword ptr [rip + __tsan_release@GOTPCREL]",
    "test rax, rax",
    "jz 3f",
    "jmp rax",
    "3:",
    "ret",
);

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
unsafe extern "C" {
    fn iox2_tsan_acquire_hook(addr: *mut u8);
    fn iox2_tsan_release_hook(addr: *mut u8);
}

#[repr(align(64))]
struct TsanSlot(UnsafeCell<[u64; 8]>);
unsafe impl Sync for TsanSlot {}

static TSAN_SLOTS: [TsanSlot; 1024] = [const { TsanSlot(UnsafeCell::new([0; 8])) }; 1024];

#[inline(always)]
fn tsan_slot(addr: usize) -> *mut u8 {
    let idx = (addr >> 2) & 1023;
    TSAN_SLOTS[idx].0.get().cast::<u8>()
}

#[inline(always)]
fn tsan_acquire(addr: usize) {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    unsafe {
        iox2_tsan_acquire_hook(tsan_slot(addr));
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    let _ = addr;
}

#[inline(always)]
fn tsan_release(addr: usize) {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    unsafe {
        iox2_tsan_release_hook(tsan_slot(addr));
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    let _ = addr;
}

#[inline]
pub fn fence(order: Ordering) {
    match order {
        Ordering::Release => tsan_release(0),
        Ordering::AcqRel | Ordering::SeqCst => {
            tsan_release(0);
        }
        _ => {}
    }
    iceoryx2_pal_concurrency_sync::atomic::fence(order);
    match order {
        Ordering::Acquire | Ordering::AcqRel | Ordering::SeqCst => tsan_acquire(0),
        _ => {}
    }
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicBool(internal::AtomicBool);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicU8(internal::AtomicU8);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicU16(internal::AtomicU16);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicU32(internal::AtomicU32);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicU64(internal::AtomicU64);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicUsize(internal::AtomicUsize);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicI8(internal::AtomicI8);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicI16(internal::AtomicI16);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicI32(internal::AtomicI32);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicI64(internal::AtomicI64);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct AtomicIsize(internal::AtomicIsize);

macro_rules! Impl {
    ($type_name:ident, $base_type:ident $(, $is_int:ident)?) => {
        impl $type_name {
            #[inline]
            pub const fn new(v: $base_type) -> Self {
                Self(internal::$type_name::new(v))
            }

            #[inline]
            pub fn load(&self, order: Ordering) -> $base_type {
                let v = self.0.load(order);
                tsan_acquire(self as *const Self as usize);
                v
            }

            #[inline]
            pub fn store(&self, val: $base_type, order: Ordering) {
                tsan_release(self as *const Self as usize);
                self.0.store(val, order);
            }

            #[inline]
            pub fn swap(&self, val: $base_type, order: Ordering) -> $base_type {
                let addr = self as *const Self as usize;
                tsan_release(addr);
                let v = self.0.swap(val, order);
                tsan_acquire(addr);
                v
            }

            #[inline]
            pub fn compare_exchange(
                &self,
                current: $base_type,
                new: $base_type,
                success: Ordering,
                failure: Ordering,
            ) -> Result<$base_type, $base_type> {
                let addr = self as *const Self as usize;
                tsan_release(addr);
                let res = self.0.compare_exchange(current, new, success, failure);
                tsan_acquire(addr);
                res
            }

            #[inline]
            pub fn compare_exchange_weak(
                &self,
                current: $base_type,
                new: $base_type,
                success: Ordering,
                failure: Ordering,
            ) -> Result<$base_type, $base_type> {
                let addr = self as *const Self as usize;
                tsan_release(addr);
                let res = self.0.compare_exchange_weak(current, new, success, failure);
                tsan_acquire(addr);
                res
            }

            #[inline]
            pub fn fetch_and(&self, val: $base_type, order: Ordering) -> $base_type {
                let addr = self as *const Self as usize;
                tsan_release(addr);
                let v = self.0.fetch_and(val, order);
                tsan_acquire(addr);
                v
            }

            #[inline]
            pub fn fetch_nand(&self, val: $base_type, order: Ordering) -> $base_type {
                let addr = self as *const Self as usize;
                tsan_release(addr);
                let v = self.0.fetch_nand(val, order);
                tsan_acquire(addr);
                v
            }

            #[inline]
            pub fn fetch_or(&self, val: $base_type, order: Ordering) -> $base_type {
                let addr = self as *const Self as usize;
                tsan_release(addr);
                let v = self.0.fetch_or(val, order);
                tsan_acquire(addr);
                v
            }

            #[inline]
            pub fn fetch_xor(&self, val: $base_type, order: Ordering) -> $base_type {
                let addr = self as *const Self as usize;
                tsan_release(addr);
                let v = self.0.fetch_xor(val, order);
                tsan_acquire(addr);
                v
            }

            #[inline]
            pub fn fetch_update<F: FnMut($base_type) -> Option<$base_type>>(
                &self,
                set_order: Ordering,
                fetch_order: Ordering,
                f: F,
            ) -> Result<$base_type, $base_type> {
                let addr = self as *const Self as usize;
                tsan_release(addr);
                let res = self.0.fetch_update(set_order, fetch_order, f);
                tsan_acquire(addr);
                res
            }

            $(
                #[inline]
                pub fn fetch_add(&self, val: $base_type, order: Ordering) -> $base_type {
                    let _ = stringify!($is_int);
                    let addr = self as *const Self as usize;
                    tsan_release(addr);
                    let v = self.0.fetch_add(val, order);
                    tsan_acquire(addr);
                    v
                }

                #[inline]
                pub fn fetch_sub(&self, val: $base_type, order: Ordering) -> $base_type {
                    let addr = self as *const Self as usize;
                    tsan_release(addr);
                    let v = self.0.fetch_sub(val, order);
                    tsan_acquire(addr);
                    v
                }

                #[inline]
                pub fn fetch_max(&self, val: $base_type, order: Ordering) -> $base_type {
                    let addr = self as *const Self as usize;
                    tsan_release(addr);
                    let v = self.0.fetch_max(val, order);
                    tsan_acquire(addr);
                    v
                }

                #[inline]
                pub fn fetch_min(&self, val: $base_type, order: Ordering) -> $base_type {
                    let addr = self as *const Self as usize;
                    tsan_release(addr);
                    let v = self.0.fetch_min(val, order);
                    tsan_acquire(addr);
                    v
                }
            )?
        }

        impl Deref for $type_name {
            type Target = internal::$type_name;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $type_name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl PlacementDefault for $type_name {
            unsafe fn placement_default(ptr: *mut Self) {
                unsafe { ptr.write(<$type_name>::default()) }
            }
        }

        unsafe impl ZeroCopySend for $type_name {}
        unsafe impl Zeroable for $type_name {}
    };
}

Impl!(AtomicBool, bool);
Impl!(AtomicU8, u8, int);
Impl!(AtomicU16, u16, int);
Impl!(AtomicU32, u32, int);
Impl!(AtomicU64, u64, int);
Impl!(AtomicUsize, usize, int);
Impl!(AtomicI8, i8, int);
Impl!(AtomicI16, i16, int);
Impl!(AtomicI32, i32, int);
Impl!(AtomicI64, i64, int);
Impl!(AtomicIsize, isize, int);

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Atomic<T: internal::AtomicInteger>(internal::Atomic<T>);

//
// Implemented manually due to generic parameter

impl<T: internal::AtomicInteger> Atomic<T> {}

impl<T: internal::AtomicInteger> Atomic<T> {
    #[inline]
    pub const fn new(v: T) -> Self {
        Self(internal::Atomic::new(v))
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

impl<T: internal::AtomicInteger> Deref for Atomic<T> {
    type Target = internal::Atomic<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: internal::AtomicInteger> DerefMut for Atomic<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: internal::AtomicInteger> PlacementDefault for Atomic<T> {
    unsafe fn placement_default(ptr: *mut Self) {
        unsafe { ptr.write(<Atomic<T>>::default()) }
    }
}

unsafe impl<T: internal::AtomicInteger> ZeroCopySend for Atomic<T> {}

pub mod internal {
    pub use iceoryx2_pal_concurrency_sync::atomic::Atomic;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicBool;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicI8;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicI16;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicI32;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicI64;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicIsize;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicU8;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicU16;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicU32;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicU64;
    pub use iceoryx2_pal_concurrency_sync::atomic::AtomicUsize;

    #[doc(hidden)]
    pub use iceoryx2_pal_concurrency_sync::atomic::internal::AtomicInteger;
}
