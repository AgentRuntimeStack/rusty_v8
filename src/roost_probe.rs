// Copyright 2026 the Roost authors. MIT license.

//! Reports the configuration of the linked V8 static library, so that an
//! embedder can refuse to run on a build other than the one it requires.
//! Cargo features only describe what was requested; these values come from
//! the archive itself.

use crate::Isolate;
use crate::isolate::RealIsolate;
use std::ffi::CStr;
use std::ffi::c_char;

/// Layout version shared with `roost__*` in `binding.cc`.
pub const ABI_VERSION: u32 = 1;

#[repr(C)]
struct RawEngineConfig {
  abi_version: u32,
  struct_size: u32,
  sandbox: bool,
  compress_pointers: bool,
  multiple_cages: bool,
  shared_cage: bool,
  external_code_space: bool,
  smi_31bit: bool,
  enable_checks: bool,
  can_create_new_groups: bool,
  sandbox_prohibit_insecure_mode: bool,
  freeze_flags_after_init: bool,
  sandbox_size: usize,
  typed_array_max_byte_length: usize,
  version: *const c_char,
}
const _: () = assert!(std::mem::size_of::<RawEngineConfig>() == 48);

#[repr(C)]
struct RawSandboxInfo {
  abi_version: u32,
  struct_size: u32,
  address_space_base: usize,
  address_space_size: usize,
  size: usize,
  reservation_size: usize,
  configured_securely: bool,
  mode: u8,
}
const _: () = assert!(std::mem::size_of::<RawSandboxInfo>() == 48);

unsafe extern "C" {
  fn roost__GetEngineConfig(out: *mut RawEngineConfig);
  fn roost__Isolate__GetSandboxInfo(
    isolate: *mut RealIsolate,
    out: *mut RawSandboxInfo,
  ) -> bool;
}

/// Configuration of the linked engine. The first group of fields are the
/// configuration macros `binding.cc` was compiled with inside the archive;
/// the rest are answered by the V8 library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineConfig {
  pub abi_version: u32,
  pub struct_size: u32,
  pub sandbox: bool,
  pub compress_pointers: bool,
  pub multiple_cages: bool,
  pub shared_cage: bool,
  pub external_code_space: bool,
  pub smi_31bit: bool,
  pub enable_checks: bool,
  /// `v8::internal::kSandboxSize` as compiled into the binding, or 0.
  pub sandbox_size: usize,
  /// `v8::TypedArray::kMaxByteLength` as compiled into the binding.
  pub typed_array_max_byte_length: usize,
  pub can_create_new_groups: bool,
  pub sandbox_prohibit_insecure_mode: bool,
  pub freeze_flags_after_init: bool,
  /// `v8::V8::GetVersion()`.
  pub version: String,
}

/// The V8 Sandbox of one isolate's group, from the public sandbox queries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxInfo {
  pub abi_version: u32,
  pub struct_size: u32,
  /// Virtual address space backing the sandbox, guard regions included.
  pub address_space_base: usize,
  pub address_space_size: usize,
  pub size: usize,
  pub reservation_size: usize,
  pub configured_securely: bool,
  /// `v8::V8::SandboxMode` as its underlying integer; 0 is `kSecure`.
  pub mode: u8,
}

/// Can be called before V8 is initialized; the flag fields then show the
/// values that initialization will use.
pub fn engine_config() -> EngineConfig {
  let mut raw = std::mem::MaybeUninit::<RawEngineConfig>::uninit();
  let raw = unsafe {
    roost__GetEngineConfig(raw.as_mut_ptr());
    raw.assume_init()
  };
  let version = if raw.version.is_null() {
    String::new()
  } else {
    unsafe { CStr::from_ptr(raw.version) }
      .to_string_lossy()
      .into_owned()
  };
  EngineConfig {
    abi_version: raw.abi_version,
    struct_size: raw.struct_size,
    sandbox: raw.sandbox,
    compress_pointers: raw.compress_pointers,
    multiple_cages: raw.multiple_cages,
    shared_cage: raw.shared_cage,
    external_code_space: raw.external_code_space,
    smi_31bit: raw.smi_31bit,
    enable_checks: raw.enable_checks,
    sandbox_size: raw.sandbox_size,
    typed_array_max_byte_length: raw.typed_array_max_byte_length,
    can_create_new_groups: raw.can_create_new_groups,
    sandbox_prohibit_insecure_mode: raw.sandbox_prohibit_insecure_mode,
    freeze_flags_after_init: raw.freeze_flags_after_init,
    version,
  }
}

impl Isolate {
  /// Describes the sandbox of this isolate's group, or `None` if the linked
  /// V8 was built without the sandbox.
  pub fn sandbox_info(&self) -> Option<SandboxInfo> {
    let mut raw = std::mem::MaybeUninit::<RawSandboxInfo>::uninit();
    let (present, raw) = unsafe {
      let present =
        roost__Isolate__GetSandboxInfo(self.as_real_ptr(), raw.as_mut_ptr());
      (present, raw.assume_init())
    };
    present.then_some(SandboxInfo {
      abi_version: raw.abi_version,
      struct_size: raw.struct_size,
      address_space_base: raw.address_space_base,
      address_space_size: raw.address_space_size,
      size: raw.size,
      reservation_size: raw.reservation_size,
      configured_securely: raw.configured_securely,
      mode: raw.mode,
    })
  }
}
