// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Vendored venue specs and the walker that lays them out.
//!
//! `tests/fixtures/pools/solana/<slug>/spec.json` is the venue's published account layout in
//! Anchor IDL JSON form: the IDL itself where the venue publishes one, otherwise a transcription
//! of the struct source in the same form. `spec-source.json` beside it records where the bytes
//! came from (url, repository commit or package version, file path, licence, retrieval date),
//! the program id, and the SHA-256 of the vendored `spec.json`. The spec is the independent
//! truth the venue layouts are tested against, so nothing in this file calls the layouts.
//!
//! # The walker
//!
//! [`Spec::from_idl`] reads both IDL forms: the pre-0.30 form (accounts carry their struct,
//! `publicKey`, no explicit discriminator) and the 0.30+ form (accounts name a type, `pubkey`,
//! explicit `discriminator`, per-type `repr` and `serialization`).
//!
//! * Fixed-size types are integers, `bool`, floats, `pubkey`, fixed arrays, unit-only enums
//!   (one byte) and nested defined structs. Explicit padding fields are ordinary fields.
//! * A struct with `repr: c` that is not packed is laid out with C alignment (`u128` aligns to
//!   [`U128_ALIGN`]); every other struct is contiguous, which is Borsh and packed `repr(C)`.
//! * The discriminator is the IDL's explicit one, which may be empty for a program that is not
//!   Anchor; otherwise it is `sha256("account:<Name>")[..8]`.
//! * A variable-size field (`vec`, `string`, `bytes`, `option`, an enum with data) ends the
//!   computable prefix. Fields after it have no offset and cannot be verified from the spec.
//!
//! # Size disagreements
//!
//! A recorded account must be exactly its spec's size, or at least the spec's computable prefix
//! when the spec is variable. A disagreement is a finding about the spec or the recording, never
//! something to adjust away: it is listed in [`SIZE_DISAGREEMENTS`], which fails when a listed
//! entry stops disagreeing and when an unlisted one appears.

// The venue modules read through the whole API; the tests of this module use part of it.
#![allow(dead_code)]

use crate::common::pool_cases::{fixtures_dir, load_venue_cases};
use crate::common::solana_pools::{SolanaCase, CHAIN};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::path::PathBuf;

/// Alignment of `u128` in a C-aligned struct: Rust on 64-bit targets aligns it to 16 bytes.
const U128_ALIGN: usize = 16;

/// Nesting limit for defined types, which guards a self-referencing spec.
const MAX_DEPTH: usize = 16;

/// Bytes of the Anchor account discriminator.
const ANCHOR_DISCRIMINATOR_LEN: usize = 8;

/// A scalar read out of account bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scalar {
    Unsigned(u128),
    Signed(i128),
    Bool(bool),
    /// A public key, a float, or an array of bytes.
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
enum Kind {
    Unsigned,
    Signed,
    Bool,
    Bytes,
    Array { element: Box<Node>, len: usize },
    Struct { fields: Vec<Field> },
    Variable,
}

#[derive(Debug, Clone)]
struct Node {
    kind: Kind,
    /// Bytes the type occupies, `None` when its size depends on the data.
    size: Option<usize>,
    /// Bytes of the type that are placed before the first variable-size field.
    prefix: usize,
    align: usize,
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    offset: usize,
    node: Node,
}

/// One account type of a spec: its discriminator and the layout of its struct.
#[derive(Debug, Clone)]
pub struct AccountLayout {
    name: String,
    discriminator: Vec<u8>,
    root: Node,
}

impl AccountLayout {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn discriminator(&self) -> &[u8] {
        &self.discriminator
    }

    /// Bytes of the account struct after the discriminator, `None` when it is variable.
    pub fn size(&self) -> Option<usize> {
        self.root.size
    }

    /// Bytes of the whole account, discriminator included, `None` when it is variable.
    pub fn data_len(&self) -> Option<usize> {
        self.root.size.map(|size| self.discriminator.len() + size)
    }

    /// Bytes of the whole account that the spec places, up to the first variable-size field.
    pub fn prefix_len(&self) -> usize {
        self.discriminator.len() + self.root.prefix
    }

    /// The byte range of a field within the whole account data, discriminator included. The path
    /// names fields with `.` and array elements with `[i]`: `fees.trade_fee_numerator`,
    /// `reward_infos[1].mint`.
    pub fn field_range(&self, path: &str) -> Result<Range<usize>, String> {
        let (offset, node) = self.resolve(path)?;
        let size = node
            .size
            .ok_or_else(|| format!("{}.{path} has no fixed size", self.name))?;
        let start = self.discriminator.len() + offset;
        Ok(start..start + size)
    }

    /// Read the scalar at `path` out of the whole account data.
    pub fn read(&self, data: &[u8], path: &str) -> Result<Scalar, String> {
        let (offset, node) = self.resolve(path)?;
        let size = node
            .size
            .ok_or_else(|| format!("{}.{path} has no fixed size", self.name))?;
        let start = self.discriminator.len() + offset;
        let bytes = data.get(start..start + size).ok_or_else(|| {
            format!(
                "{}.{path} needs bytes {start}..{} of {}",
                self.name,
                start + size,
                data.len()
            )
        })?;
        match &node.kind {
            Kind::Unsigned => Ok(Scalar::Unsigned(unsigned_le(bytes))),
            Kind::Signed => Ok(Scalar::Signed(signed_le(bytes))),
            Kind::Bool => Ok(Scalar::Bool(bytes[0] != 0)),
            Kind::Bytes => Ok(Scalar::Bytes(bytes.to_vec())),
            Kind::Array { element, .. } if matches!(element.kind, Kind::Bytes) => {
                Ok(Scalar::Bytes(bytes.to_vec()))
            }
            _ => Err(format!("{}.{path} is not a scalar", self.name)),
        }
    }

    fn resolve(&self, path: &str) -> Result<(usize, &Node), String> {
        let mut node = &self.root;
        let mut offset = 0;
        for segment in path.split('.') {
            let (name, indexes) = split_indexes(segment).ok_or_else(|| {
                format!("{}.{path}: malformed path segment `{segment}`", self.name)
            })?;
            let Kind::Struct { fields } = &node.kind else {
                return Err(format!(
                    "{}.{path}: `{name}` is not inside a struct",
                    self.name
                ));
            };
            let field = fields
                .iter()
                .find(|field| field.name == name)
                .ok_or_else(|| {
                    format!(
                        "{}.{path}: no field `{name}` before the first variable-size field",
                        self.name
                    )
                })?;
            offset += field.offset;
            node = &field.node;
            for index in indexes {
                let Kind::Array { element, len } = &node.kind else {
                    return Err(format!("{}.{path}: `{name}` is not an array", self.name));
                };
                if index >= *len {
                    return Err(format!("{}.{path}: index {index} is past {len}", self.name));
                }
                offset += index * element.size.expect("array elements have a fixed size");
                node = element;
            }
        }
        Ok((offset, node))
    }
}

/// The account types of one spec.
#[derive(Debug, Clone)]
pub struct Spec {
    accounts: Vec<AccountLayout>,
}

impl Spec {
    /// Lay out every account of an IDL.
    pub fn from_idl(idl: &Value) -> Result<Self, String> {
        let types: BTreeMap<String, &Value> = idl
            .get("types")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|def| Some((def.get("name")?.as_str()?.to_owned(), def)))
            .collect();
        let walker = Walker { types };
        let mut accounts = Vec::new();
        for account in idl
            .get("accounts")
            .and_then(Value::as_array)
            .ok_or("the spec lists no accounts")?
        {
            let name = account
                .get("name")
                .and_then(Value::as_str)
                .ok_or("an account has no name")?;
            let def = match account.get("type") {
                Some(_) => account,
                None => walker
                    .types
                    .get(name)
                    .copied()
                    .ok_or_else(|| format!("account {name} has no struct in the spec"))?,
            };
            let discriminator = match account.get("discriminator") {
                Some(bytes) => bytes
                    .as_array()
                    .ok_or_else(|| format!("account {name}: discriminator is not an array"))?
                    .iter()
                    .map(|byte| byte.as_u64().and_then(|b| u8::try_from(b).ok()))
                    .collect::<Option<Vec<u8>>>()
                    .ok_or_else(|| format!("account {name}: discriminator is not bytes"))?,
                None => {
                    Sha256::digest(format!("account:{name}"))[..ANCHOR_DISCRIMINATOR_LEN].to_vec()
                }
            };
            let root = walker.struct_node(def, 0)?;
            accounts.push(AccountLayout {
                name: name.to_owned(),
                discriminator,
                root,
            });
        }
        Ok(Self { accounts })
    }

    pub fn accounts(&self) -> &[AccountLayout] {
        &self.accounts
    }

    pub fn account(&self, name: &str) -> &AccountLayout {
        self.accounts
            .iter()
            .find(|account| account.name == name)
            .unwrap_or_else(|| panic!("the spec has no account {name}"))
    }

    /// The account type whose discriminator opens `data`. A program with no discriminators has
    /// one account type, which matches any data.
    pub fn account_for(&self, data: &[u8]) -> Result<&AccountLayout, String> {
        let mut matches: Vec<&AccountLayout> = self
            .accounts
            .iter()
            .filter(|account| data.starts_with(&account.discriminator))
            .collect();
        let longest = matches
            .iter()
            .map(|account| account.discriminator.len())
            .max()
            .ok_or_else(|| {
                format!(
                    "no account of the spec has the discriminator {}",
                    hex(&data[..data.len().min(ANCHOR_DISCRIMINATOR_LEN)])
                )
            })?;
        matches.retain(|account| account.discriminator.len() == longest);
        match matches.as_slice() {
            [account] => Ok(account),
            _ => Err(format!(
                "{} account types of the spec share the discriminator {}",
                matches.len(),
                hex(&data[..longest])
            )),
        }
    }
}

struct Walker<'a> {
    types: BTreeMap<String, &'a Value>,
}

impl Walker<'_> {
    fn node(&self, ty: &Value, depth: usize) -> Result<Node, String> {
        if depth > MAX_DEPTH {
            return Err("the spec nests types deeper than the walker follows".to_owned());
        }
        match ty {
            Value::String(name) => Ok(primitive(name)),
            Value::Object(object) => {
                if let Some(array) = object.get("array").and_then(Value::as_array) {
                    let [element, len] = array.as_slice() else {
                        return Err("an array type needs an element and a length".to_owned());
                    };
                    let len = len
                        .as_u64()
                        .and_then(|len| usize::try_from(len).ok())
                        .ok_or("an array length is not a number")?;
                    let element = self.node(element, depth + 1)?;
                    return Ok(array_node(element, len));
                }
                if let Some(defined) = object.get("defined") {
                    let name = defined
                        .as_str()
                        .or_else(|| defined.get("name").and_then(Value::as_str))
                        .ok_or("a defined type has no name")?;
                    let def = self
                        .types
                        .get(name)
                        .ok_or_else(|| format!("the spec does not define {name}"))?;
                    return self.defined_node(def, depth + 1);
                }
                if ["vec", "option", "coption", "hashMap", "bTreeMap"]
                    .iter()
                    .any(|key| object.contains_key(*key))
                {
                    return Ok(variable());
                }
                Err(format!("unsupported type {ty}"))
            }
            _ => Err(format!("unsupported type {ty}")),
        }
    }

    fn defined_node(&self, def: &Value, depth: usize) -> Result<Node, String> {
        let ty = def.get("type").ok_or("a defined type has no body")?;
        match ty.get("kind").and_then(Value::as_str) {
            Some("struct") => self.struct_node(def, depth),
            Some("enum") => {
                let unit_only = ty
                    .get("variants")
                    .and_then(Value::as_array)
                    .ok_or("an enum has no variants")?
                    .iter()
                    .all(|variant| {
                        variant
                            .get("fields")
                            .and_then(Value::as_array)
                            .is_none_or(Vec::is_empty)
                    });
                Ok(if unit_only {
                    scalar(Kind::Unsigned, 1, 1)
                } else {
                    variable()
                })
            }
            Some("type") => self.node(ty.get("alias").ok_or("an alias has no target")?, depth),
            other => Err(format!("unsupported type kind {other:?}")),
        }
    }

    fn struct_node(&self, def: &Value, depth: usize) -> Result<Node, String> {
        let ty = def.get("type").ok_or("a struct has no body")?;
        let repr = def.get("repr");
        let is_c = repr
            .and_then(|repr| repr.get("kind"))
            .and_then(Value::as_str)
            == Some("c");
        let packed = repr
            .and_then(|repr| repr.get("packed"))
            .is_some_and(|packed| packed.as_bool() != Some(false));
        let aligned = is_c && !packed;
        let declared: &[Value] = ty
            .get("fields")
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice);
        let mut fields = Vec::new();
        let mut offset = 0;
        let mut struct_align = 1;
        let mut variable_at = None;
        for (index, field) in declared.iter().enumerate() {
            // Named fields are objects; tuple fields are bare types.
            let (name, field_type) = match field.get("type") {
                Some(field_type) => (
                    field
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or("a field has no name")?
                        .to_owned(),
                    field_type,
                ),
                None => (index.to_string(), field),
            };
            let node = self.node(field_type, depth + 1)?;
            let align = if aligned { node.align } else { 1 };
            offset = round_up(offset, align);
            struct_align = struct_align.max(align);
            let size = node.size;
            fields.push(Field { name, offset, node });
            match size {
                Some(size) => offset += size,
                None => {
                    variable_at = Some(offset);
                    break;
                }
            }
        }
        let (size, prefix) = match variable_at {
            Some(at) => (None, at),
            None => {
                let size = round_up(offset, struct_align);
                (Some(size), size)
            }
        };
        Ok(Node {
            kind: Kind::Struct { fields },
            size,
            prefix,
            align: struct_align,
        })
    }
}

fn primitive(name: &str) -> Node {
    match name {
        "bool" => scalar(Kind::Bool, 1, 1),
        "u8" => scalar(Kind::Unsigned, 1, 1),
        "u16" => scalar(Kind::Unsigned, 2, 2),
        "u32" => scalar(Kind::Unsigned, 4, 4),
        "u64" => scalar(Kind::Unsigned, 8, 8),
        "u128" => scalar(Kind::Unsigned, 16, U128_ALIGN),
        "i8" => scalar(Kind::Signed, 1, 1),
        "i16" => scalar(Kind::Signed, 2, 2),
        "i32" => scalar(Kind::Signed, 4, 4),
        "i64" => scalar(Kind::Signed, 8, 8),
        "i128" => scalar(Kind::Signed, 16, U128_ALIGN),
        "f32" => scalar(Kind::Bytes, 4, 4),
        "f64" => scalar(Kind::Bytes, 8, 8),
        "pubkey" | "publicKey" => scalar(Kind::Bytes, 32, 1),
        _ => variable(),
    }
}

fn scalar(kind: Kind, size: usize, align: usize) -> Node {
    Node {
        kind,
        size: Some(size),
        prefix: size,
        align,
    }
}

fn variable() -> Node {
    Node {
        kind: Kind::Variable,
        size: None,
        prefix: 0,
        align: 1,
    }
}

fn array_node(element: Node, len: usize) -> Node {
    let align = element.align;
    match element.size {
        Some(each) => Node {
            size: Some(each * len),
            prefix: each * len,
            align,
            kind: Kind::Array {
                element: Box::new(element),
                len,
            },
        },
        None => variable(),
    }
}

fn round_up(value: usize, align: usize) -> usize {
    value.div_ceil(align) * align
}

/// `name[1][2]` as `("name", [1, 2])`.
fn split_indexes(segment: &str) -> Option<(&str, Vec<usize>)> {
    let Some((name, mut rest)) = segment.split_once('[') else {
        return Some((segment, Vec::new()));
    };
    let mut indexes = Vec::new();
    loop {
        let (index, tail) = rest.split_once(']')?;
        indexes.push(index.parse().ok()?);
        match tail {
            "" => return Some((name, indexes)),
            tail => rest = tail.strip_prefix('[')?,
        }
    }
}

fn unsigned_le(bytes: &[u8]) -> u128 {
    bytes
        .iter()
        .rev()
        .fold(0u128, |value, byte| (value << 8) | u128::from(*byte))
}

fn signed_le(bytes: &[u8]) -> i128 {
    let negative = bytes.last().is_some_and(|byte| byte & 0x80 != 0);
    let mut extended = [if negative { 0xff } else { 0 }; 16];
    extended[..bytes.len()].copy_from_slice(bytes);
    i128::from_le_bytes(extended)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A venue's vendored spec together with its provenance record.
pub struct VenueSpec {
    pub spec: Spec,
    pub source: Value,
}

impl VenueSpec {
    pub fn program_id(&self) -> &str {
        self.source["program_id"]
            .as_str()
            .expect("spec-source.json records a program id")
    }
}

/// The directory of one venue's spec, source record and recorded cases.
pub fn venue_dir(slug: &str) -> PathBuf {
    fixtures_dir().join(CHAIN).join(slug)
}

/// The slug of every venue directory that holds a spec or a spec source, sorted.
pub fn spec_slugs() -> Vec<String> {
    let root = fixtures_dir().join(CHAIN);
    let mut slugs: Vec<String> = std::fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("read {}: {e}", root.display()))
        .map(|entry| entry.expect("read venue directory entry").path())
        .filter(|path| path.join("spec.json").is_file() || path.join("spec-source.json").is_file())
        .filter_map(|path| Some(path.file_name()?.to_str()?.to_owned()))
        .collect();
    slugs.sort();
    slugs
}

fn read_json(path: &std::path::Path) -> Value {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// Load and lay out the vendored spec of a venue.
pub fn load_venue_spec(slug: &str) -> VenueSpec {
    let dir = venue_dir(slug);
    let spec = Spec::from_idl(&read_json(&dir.join("spec.json")))
        .unwrap_or_else(|e| panic!("{slug}: {e}"));
    VenueSpec {
        spec,
        source: read_json(&dir.join("spec-source.json")),
    }
}

/// Every recorded case of a venue.
pub fn venue_cases(slug: &str) -> Vec<(String, SolanaCase)> {
    load_venue_cases(CHAIN, slug)
}

#[test]
fn every_spec_matches_its_recorded_source_digest() {
    let slugs = spec_slugs();
    assert!(!slugs.is_empty(), "no venue has a vendored spec");
    let mut problems = Vec::new();
    for slug in &slugs {
        let dir = venue_dir(slug);
        let (spec_path, source_path) = (dir.join("spec.json"), dir.join("spec-source.json"));
        if !spec_path.is_file() || !source_path.is_file() {
            problems.push(format!(
                "{slug}: spec.json and spec-source.json come together"
            ));
            continue;
        }
        let source = read_json(&source_path);
        let text = |key: &str| {
            source
                .get(key)
                .and_then(Value::as_str)
                .filter(|v| !v.is_empty())
        };
        for key in [
            "slug",
            "program_id",
            "source_kind",
            "url",
            "path",
            "licence",
            "retrieved",
            "sha256",
        ] {
            if text(key).is_none() {
                problems.push(format!("{slug}: spec-source.json has no `{key}`"));
            }
        }
        if text("commit").is_none() && text("package").is_none() {
            problems.push(format!(
                "{slug}: spec-source.json pins neither a commit nor a package"
            ));
        }
        if text("slug").is_some_and(|recorded| recorded != slug) {
            problems.push(format!("{slug}: spec-source.json names another venue"));
        }
        let bytes = std::fs::read(&spec_path).expect("read spec.json");
        let actual = hex(&Sha256::digest(&bytes));
        match text("sha256") {
            Some(recorded) if recorded == actual => {}
            Some(recorded) => problems.push(format!(
                "{slug}: spec.json digest is {actual}, spec-source.json records {recorded}"
            )),
            None => {}
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_spec_names_the_program_its_source_records() {
    let mut problems = Vec::new();
    for slug in spec_slugs() {
        let venue = load_venue_spec(&slug);
        let idl = read_json(&venue_dir(&slug).join("spec.json"));
        let published = idl
            .get("address")
            .or_else(|| {
                idl.get("metadata")
                    .and_then(|metadata| metadata.get("address"))
            })
            .and_then(Value::as_str);
        if published.is_some_and(|address| address != venue.program_id()) {
            problems.push(format!(
                "{slug}: the spec is for {published:?}, spec-source.json records {}",
                venue.program_id()
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Recorded accounts whose length disagrees with their vendored spec, as `(slug, account type,
/// spec length, recorded length)`. Each is a finding about the published spec or the recording,
/// not a tolerance: the account is a different size from what the program's own published layout
/// says, so the bytes past the spec's last field are unverified. The list only shrinks; an entry
/// is removed in the change that resolves its disagreement.
///
/// * `meteora_dlmm` `Oracle`: the IDL describes the 24-byte header; the account carries 100
///   observation samples of 32 bytes after it, which the IDL does not describe.
/// * `moonit_amm` `ConfigAccount` and `CurveAccount`: the accounts are allocated far past the
///   fields the IDL lists; the bytes after the last listed field are zero.
/// * `pumpfun_amm` `Pool` is 30 bytes past the IDL and `GlobalConfig` 9 bytes short of it.
/// * `pumpfun_legacy` `BondingCurve` is 26 bytes past the IDL and `Global` 42 bytes short of it.
const SIZE_DISAGREEMENTS: &[(&str, &str, usize, usize)] = &[
    ("moonit_amm", "ConfigAccount", 227, 403),
    ("moonit_amm", "CurveAccount", 84, 409),
    ("pumpfun_amm", "GlobalConfig", 949, 940),
    ("pumpfun_amm", "Pool", 271, 301),
    ("pumpfun_legacy", "BondingCurve", 125, 151),
    ("pumpfun_legacy", "Global", 1087, 1045),
];

#[test]
fn every_spec_account_size_matches_its_recorded_accounts() {
    let mut problems = Vec::new();
    let mut found = BTreeSet::new();
    let mut checked = 0;
    for slug in spec_slugs() {
        let venue = load_venue_spec(&slug);
        for (case_name, case) in venue_cases(&slug) {
            for (address, account) in &case.accounts {
                if account.owner != venue.program_id() {
                    continue;
                }
                checked += 1;
                let layout = match venue.spec.account_for(&account.data) {
                    Ok(layout) => layout,
                    Err(e) => {
                        problems.push(format!("{slug}/{case_name}: account {address}: {e}"));
                        continue;
                    }
                };
                // A variable-size account is checked as far as the spec computes it.
                let length = account.data.len();
                let (spec_length, fits) = match layout.data_len() {
                    Some(exact) => (exact, exact == length),
                    None => (layout.prefix_len(), length >= layout.prefix_len()),
                };
                if !fits {
                    found.insert((slug.clone(), layout.name().to_owned(), spec_length, length));
                }
            }
        }
    }
    assert!(
        checked > 0,
        "no recorded account is owned by a spec's program"
    );
    let listed: BTreeSet<(String, String, usize, usize)> = SIZE_DISAGREEMENTS
        .iter()
        .map(|(slug, name, spec, recorded)| (slug.to_string(), name.to_string(), *spec, *recorded))
        .collect();
    for (slug, name, spec, recorded) in found.difference(&listed) {
        problems.push(format!(
            "{slug}: a recorded {name} holds {recorded} bytes, the spec says {spec}"
        ));
    }
    for (slug, name, spec, recorded) in listed.difference(&found) {
        problems.push(format!(
            "{slug}: {name} ({spec} in the spec, {recorded} recorded) no longer disagrees; remove it \
             from SIZE_DISAGREEMENTS"
        ));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// An IDL in the 0.30 form with explicit discriminators.
fn synthetic_idl() -> Value {
    serde_json::json!({
        "address": "11111111111111111111111111111111",
        "accounts": [
            {"name": "Aligned", "discriminator": [1, 2, 3, 4, 5, 6, 7, 8]},
            {"name": "Packed", "discriminator": [9, 9, 9, 9, 9, 9, 9, 9]},
            {"name": "Borsh", "discriminator": [3, 3, 3, 3, 3, 3, 3, 3]},
            {"name": "Open", "discriminator": []},
            {"name": "Growing", "discriminator": [4, 4, 4, 4, 4, 4, 4, 4]}
        ],
        "types": [
            {"name": "Aligned", "serialization": "bytemuck", "repr": {"kind": "c"},
             "type": {"kind": "struct", "fields": [
                {"name": "flag", "type": "u8"},
                {"name": "amount", "type": "u64"},
                {"name": "tag", "type": "u8"},
                {"name": "wide", "type": "u128"},
                {"name": "inner", "type": {"defined": {"name": "Inner"}}},
                {"name": "keys", "type": {"array": ["pubkey", 2]}},
                {"name": "tail", "type": "u8"}
             ]}},
            {"name": "Inner", "serialization": "bytemuck", "repr": {"kind": "c"},
             "type": {"kind": "struct", "fields": [
                {"name": "small", "type": "u16"},
                {"name": "byte", "type": "u8"}
             ]}},
            {"name": "Packed", "serialization": "bytemuckunsafe",
             "repr": {"kind": "c", "packed": true},
             "type": {"kind": "struct", "fields": [
                {"name": "flag", "type": "u8"},
                {"name": "amount", "type": "u64"},
                {"name": "wide", "type": "u128"}
             ]}},
            {"name": "Borsh",
             "type": {"kind": "struct", "fields": [
                {"name": "flag", "type": "u8"},
                {"name": "amount", "type": "u64"},
                {"name": "mode", "type": {"defined": {"name": "Mode"}}},
                {"name": "signed", "type": "i32"},
                {"name": "on", "type": "bool"},
                {"name": "grid", "type": {"array": [{"array": ["u16", 2]}, 3]}}
             ]}},
            {"name": "Mode", "type": {"kind": "enum", "variants": [{"name": "Off"}, {"name": "On"}]}},
            {"name": "Open", "type": {"kind": "struct", "fields": [
                {"name": "version", "type": "u8"},
                {"name": "owner", "type": "pubkey"}
            ]}},
            {"name": "Growing", "type": {"kind": "struct", "fields": [
                {"name": "head", "type": "u8"},
                {"name": "count", "type": "u32"},
                {"name": "label", "type": "string"},
                {"name": "after", "type": "u64"}
            ]}}
        ]
    })
}

#[test]
fn the_walker_lays_out_a_struct_with_known_offsets() {
    let spec = Spec::from_idl(&synthetic_idl()).expect("synthetic spec");

    // C alignment: u8 at 0, u64 at 8, u8 at 16, u128 at 32, Inner (u16, u8: size 4, align 2) at
    // 48, two public keys from 52, the tail at 116, and the struct rounded up to 16.
    let aligned = spec.account("Aligned");
    assert_eq!(aligned.discriminator(), [1, 2, 3, 4, 5, 6, 7, 8]);
    for (path, start, end) in [
        ("flag", 8, 9),
        ("amount", 16, 24),
        ("tag", 24, 25),
        ("wide", 40, 56),
        ("inner", 56, 60),
        ("inner.byte", 58, 59),
        ("keys", 60, 124),
        ("keys[1]", 92, 124),
        ("tail", 124, 125),
    ] {
        assert_eq!(aligned.field_range(path), Ok(start..end), "{path}");
    }
    assert_eq!(aligned.size(), Some(128));
    assert_eq!(aligned.data_len(), Some(136));

    // Packed repr(C) is contiguous.
    let packed = spec.account("Packed");
    assert_eq!(packed.field_range("amount"), Ok(9..17));
    assert_eq!(packed.field_range("wide"), Ok(17..33));
    assert_eq!(packed.size(), Some(25));

    // Borsh is contiguous: u8, u64, a one-byte unit enum, i32, bool, then a 3x2 grid of u16.
    let borsh = spec.account("Borsh");
    for (path, start, end) in [
        ("amount", 9, 17),
        ("mode", 17, 18),
        ("signed", 18, 22),
        ("on", 22, 23),
        ("grid", 23, 35),
        ("grid[2][1]", 33, 35),
    ] {
        assert_eq!(borsh.field_range(path), Ok(start..end), "{path}");
    }
    assert_eq!(borsh.size(), Some(27));

    // An empty discriminator puts the first field at offset zero.
    let open = spec.account("Open");
    assert_eq!(open.discriminator(), [] as [u8; 0]);
    assert_eq!(open.field_range("owner"), Ok(1..33));
    assert_eq!(open.data_len(), Some(33));

    // Reading writes what a program would store.
    let mut data = vec![0u8; borsh.data_len().unwrap()];
    data[..8].copy_from_slice(&[3; 8]);
    data[9..17].copy_from_slice(&0x0102_0304_0506_0708u64.to_le_bytes());
    data[18..22].copy_from_slice(&(-2i32).to_le_bytes());
    data[22] = 1;
    data[33..35].copy_from_slice(&513u16.to_le_bytes());
    assert_eq!(
        borsh.read(&data, "amount"),
        Ok(Scalar::Unsigned(0x0102_0304_0506_0708))
    );
    assert_eq!(borsh.read(&data, "signed"), Ok(Scalar::Signed(-2)));
    assert_eq!(borsh.read(&data, "on"), Ok(Scalar::Bool(true)));
    assert_eq!(borsh.read(&data, "grid[2][1]"), Ok(Scalar::Unsigned(513)));
    assert!(borsh.read(&data, "grid[1]").is_err());
    assert!(borsh.read(&data[..30], "grid[2][1]").is_err());
    assert!(borsh.read(&data, "grid[3]").is_err());
    assert!(borsh.read(&data, "nothing").is_err());
}

#[test]
fn the_walker_ends_the_computable_prefix_at_a_variable_field() {
    let spec = Spec::from_idl(&synthetic_idl()).expect("synthetic spec");
    let growing = spec.account("Growing");
    assert_eq!(growing.size(), None);
    assert_eq!(growing.data_len(), None);
    assert_eq!(growing.prefix_len(), 8 + 5);
    assert_eq!(growing.field_range("count"), Ok(9..13));
    assert!(growing.field_range("label").is_err());
    assert!(growing.field_range("after").is_err());
}

#[test]
fn the_walker_reads_the_pre_0_30_idl_form() {
    let idl = serde_json::json!({
        "version": "0.1.0",
        "name": "old",
        "accounts": [{"name": "Pool", "type": {"kind": "struct", "fields": [
            {"name": "owner", "type": "publicKey"},
            {"name": "count", "type": "u16"}
        ]}}]
    });
    let spec = Spec::from_idl(&idl).expect("pre-0.30 spec");
    let pool = spec.account("Pool");
    // The discriminator is sha256("account:Pool")[..8].
    assert_eq!(pool.discriminator(), [241, 154, 109, 4, 17, 177, 109, 188]);
    assert_eq!(pool.field_range("owner"), Ok(8..40));
    assert_eq!(pool.field_range("count"), Ok(40..42));
    assert_eq!(pool.data_len(), Some(42));
    let mut data = vec![0u8; 42];
    data[..8].copy_from_slice(&[241, 154, 109, 4, 17, 177, 109, 188]);
    assert_eq!(spec.account_for(&data).map(AccountLayout::name), Ok("Pool"));
    data[0] = 0;
    assert!(spec.account_for(&data).is_err());
}
