//! (Incomplete) tagged pointer and span implementations for stable deref of shared segments
#![allow(unused)]
use core::fmt;

use core::convert::{TryFrom, TryInto};
use core::ops::{Deref, DerefMut};
use core::option::Option;
use core::ptr::NonNull;


fn likely(x: bool) -> bool {
    x
}

type Tag = u16;
type Word = usize;
type SystemPtr = *mut u8;

const PTR_TAG_MASK_SHIFT: Word =
    (Word::max_value().count_ones() - Tag::max_value().count_ones()) as Word;
/// `0xffff_0000_0000_0000`
const PTR_KIND_MASK: Word = (Tag::max_value() as Word) << PTR_TAG_MASK_SHIFT;

/// `0x0000_fffff_0000_0000`
const PTR_HANDLE_MASK: Word = (Tag::max_value() as Word) << PTR_HANDLE_MASK_SHIFT;
const PTR_HANDLE_MASK_SHIFT: Word = 32;

/// `0x0000_ffff_ffff_ffff`
const PTR_VALUE_MASK: Word = !PTR_KIND_MASK;

/// `0x0000_0000_ffff_ffff`
const PTR_REGION_OFFSET_SHIFT: Word = 0;
const PTR_REGION_OFFSET_MASK: Word = !(PTR_KIND_MASK | PTR_HANDLE_MASK);
const PTR_REGION_OFFSET_MAX: Word = ((1 as Word) << PTR_REGION_OFFSET_MASK.count_ones()) - 1;


#[derive(Copy, Clone, Debug)]
pub struct BadPtrKind(usize);


#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(usize)]
#[non_exhaustive]
pub enum PtrKind {
    Native = 0x0000,

    Unknown = 0xfffe << PTR_TAG_MASK_SHIFT,
    Dangling = 0xffff << PTR_TAG_MASK_SHIFT,
}

impl PtrKind {
    pub const DANGLING: usize = PtrKind::Dangling as usize;
    pub const NATIVE: usize = PtrKind::Native as usize;
    pub const UNKNOWN: usize = PtrKind::Unknown as usize;

    #[inline]
    pub const fn into_tag(self) -> Tag {
        (((self as usize) & PTR_KIND_MASK) >> PTR_TAG_MASK_SHIFT) as Tag
    }
}

impl TryFrom<Tag> for PtrKind {
    type Error = BadPtrKind;

    fn try_from(tag: Tag) -> Result<Self, Self::Error> {
        let value = (tag as usize) << PTR_TAG_MASK_SHIFT;
        match value {
            PtrKind::NATIVE => Ok(PtrKind::Native),
            PtrKind::DANGLING => Ok(PtrKind::Dangling),
            PtrKind::UNKNOWN => Ok(PtrKind::Unknown),
            _ => Err(BadPtrKind(value)),
        }
    }
}

impl Into<Tag> for PtrKind {
    #[inline]
    fn into(self) -> Tag {
        self.into_tag()
    }
}


#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawPtr(usize);

impl RawPtr {
    pub fn new(ptr: SystemPtr) -> RawPtr {
        Self(ptr as usize)
    }

    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn null_mut() -> Self {
        Self(0)
    }

    pub const fn dangling() -> RawPtr {
        Self(core::usize::MAX)
    }

    #[inline(always)]
    pub fn to_usize(self) -> usize {
        self.as_ptr() as usize
    }

    #[inline]
    pub fn as_ptr(self) -> SystemPtr {
        let value = self.raw_value();
        let kind = value & PTR_KIND_MASK;
        let kind: PtrKind = unsafe { ::core::mem::transmute(kind) };
        match kind {
            PtrKind::Native => value as usize as SystemPtr,
            PtrKind::Unknown => panic!("Ptr::as_ptr: bad or unimplemented PtrKind {:?}", kind),
            _ => panic!("unreachable"),
        }
    }

    #[inline(always)]
    pub fn as_nonnull(self) -> NonNull<u8> {
        unsafe { NonNull::new_unchecked(self.as_ptr()) }
    }

    #[inline(always)]
    pub unsafe fn offset(self, n: isize) -> RawPtr {
        let value = self.raw_value();
        let kind = value & PTR_KIND_MASK;
        let kind: PtrKind = unsafe { core::mem::transmute(kind) };
        match kind {
            PtrKind::Native => RawPtr(((value as isize) + n) as usize),
            kind => panic!("Ptr::checked_add: bad or unimplemented PtrKind {:?}", kind),
        }
    }

    #[inline(always)]
    pub unsafe fn downcast<T>(self) -> Ptr<T> {
        Ptr::new(self)
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub fn checked_add(self, n: usize) -> Option<RawPtr> {
        let value = self.raw_value();
        let kind = value & PTR_KIND_MASK;
        let kind: PtrKind = unsafe { core::mem::transmute(kind) };
        match kind {
            PtrKind::Native => value.checked_add(n).map(RawPtr),
            kind => panic!("Ptr::checked_add: bad or unimplemented PtrKind {:?}", kind),
        }
    }

    #[inline(always)]
    pub fn kind(self) -> PtrKind {
        let kind = self.raw_value() & PTR_KIND_MASK;
        unsafe { core::mem::transmute(kind) }
    }

    #[inline(always)]
    const fn raw_value(self) -> Word {
        self.0
    }

    #[inline(always)]
    const unsafe fn handle_offset_pair(self) -> (usize, usize) {
        let value = self.raw_value();
        let handle = (value & PTR_HANDLE_MASK) >> PTR_HANDLE_MASK_SHIFT;
        let offset = (value & PTR_REGION_OFFSET_MASK) >> PTR_REGION_OFFSET_SHIFT;
        (handle, offset)
    }

    #[inline(always)]
    const fn data(self) -> Word {
        self.0 & PTR_VALUE_MASK
    }
}

impl fmt::Debug for RawPtr {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        #[allow(unreachable_patterns)]
        match self.kind() {
            PtrKind::Dangling => write!(f, "Ptr(Dangling)"),
            PtrKind::Native => write!(f, "Ptr(Native, {:p})", self.raw_value() as *mut u8),
            PtrKind::Unknown => write!(f, "Ptr(Unknown, {:p})", self.raw_value() as *mut u8),
            kind => write!(
                f,
                "Ptr(kind={}, {:p})",
                kind as usize,
                self.raw_value() as *mut u8
            ),
        }
    }
}


#[derive(Debug, Hash)]
#[repr(transparent)]
pub struct Ptr<T> {
    pub(super) raw: RawPtr,
    pub(super) _type: core::marker::PhantomData<*const T>,
}

impl<T> Ptr<T> {
    pub fn as_ptr(&self) -> *const T {
        self.raw.as_ptr() as *const _
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        self.raw.as_ptr().cast()
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self::new(self.raw)
    }
}

impl<T> Copy for Ptr<T> {}


impl<T> ::core::cmp::PartialEq<Self> for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> ::core::cmp::PartialOrd<Self> for Ptr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
        self.raw.partial_cmp(&other.raw)
    }
}

impl<T> Ptr<T> {
    #[inline(always)]
    fn new(raw: RawPtr) -> Self {
        Self {
            raw,
            _type: core::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub const fn null() -> Self {
        Self {
            raw: RawPtr::null(),
            _type: core::marker::PhantomData,
        }
    }

    #[inline(always)]
    pub fn dangling() -> Self {
        Self {
            raw: RawPtr::null(),
            _type: core::marker::PhantomData,
        }
    }


    #[inline(always)]
    pub fn is_null(self) -> bool {
        self.raw.is_null()
    }

    #[inline(always)]
    pub fn as_mut(self) -> *mut T {
        self.raw.as_ptr().cast()
    }
}

impl<T> From<*mut T> for Ptr<T> {
    fn from(p: *mut T) -> Self {
        Self::new(RawPtr::new(p.cast()))
    }
}

impl<T> From<*const T> for Ptr<T> {
    fn from(p: *const T) -> Self {
        Self::new(RawPtr::new(p as SystemPtr))
    }
}

impl<T> From<RawPtr> for Ptr<T> {
    fn from(p: RawPtr) -> Self {
        Self::new(p)
    }
}

impl<T> Deref for Ptr<T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.raw.as_ptr().cast() }
    }
}

impl<T> DerefMut for Ptr<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.raw.as_ptr().cast() }
    }
}
