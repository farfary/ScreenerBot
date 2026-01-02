//! NFT metadata handling module
//!
//! Fetches and parses Metaplex Token Metadata from the Solana blockchain.
//! Supports both on-chain metadata (name, symbol, uri) and off-chain JSON metadata (image).

mod metadata;

pub use metadata::{fetch_nft_metadata, fetch_nft_metadata_batch, NftMetadata, NftMetadataError};
