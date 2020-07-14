// This file is part of Substrate.

// Copyright (C) 2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Implementation of the `insert` subcommand

use crate::{Error, CliConfiguration, KeystoreParams, with_crypto_scheme, CryptoSchemeFlag, SharedParams, utils};
use structopt::StructOpt;
use sp_core::{crypto::KeyTypeId, Bytes};
use std::convert::TryFrom;
use futures01::Future;
use hyper::rt;
use sc_rpc::author::AuthorClient;
use jsonrpc_core_client::transports::http;
use serde::{de::DeserializeOwned, Serialize};
use sp_core::crypto::ExposeSecret;

/// The `insert` command
#[derive(Debug, StructOpt)]
#[structopt(
	name = "insert",
	about = "Insert a key to the keystore of a node."
)]
pub struct InsertCmd {
	/// The secret key URI.
	/// If the value is a file, the file content is used as URI.
	/// If not given, you will be prompted for the URI.
	#[structopt(long)]
	suri: Option<String>,

	/// Key type, examples: "gran", or "imon"
	#[structopt(long)]
	key_type: String,

	/// Node JSON-RPC endpoint, default "http://localhost:9933"
	#[structopt(long)]
	node_url: Option<String>,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub keystore_params: KeystoreParams,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub shared_params: SharedParams,

	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub crypto_scheme: CryptoSchemeFlag,
}

impl InsertCmd {
	/// Run the command
	pub fn run<H>(&self) -> Result<(), Error>
		where
			H: DeserializeOwned + Serialize + Send + Sync + 'static,
	{
		let suri = utils::read_uri(self.suri.as_ref())?;
		let password = self.keystore_params.read_password()?;
		let password = password.as_ref().map(|s| s.expose_secret().as_str());

		let public = with_crypto_scheme!(
			self.crypto_scheme.scheme,
			to_vec(&suri, password)
		)?;

		let node_url = self.node_url.as_ref()
			.map(String::as_str)
			.unwrap_or("http://localhost:9933");
		let key_type = &self.key_type;

		// Just checking
		let _key_type_id = KeyTypeId::try_from(key_type.as_str())
			.map_err(|_| {
				Error::Other("Cannot convert argument to keytype: argument should be 4-character string".into())
			})?;


		insert_key::<H>(
			&node_url,
			key_type.to_string(),
			suri,
			sp_core::Bytes(public),
		);

		Ok(())
	}
}

impl CliConfiguration for InsertCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		Some(&self.keystore_params)
	}
}

fn to_vec<P: sp_core::Pair>(uri: &str, pass: Option<&str>) -> Result<Vec<u8>, Error> {
	let p = utils::pair_from_suri::<P>(uri, pass)?;
	Ok(p.public().as_ref().to_vec())
}

fn insert_key<H>(url: &str, key_type: String, suri: String, public: Bytes)
	where
		H: DeserializeOwned + Serialize + Send + Sync + 'static,
{
	rt::run(
		http::connect(&url)
			.and_then(|client: AuthorClient<H, H>| {
				client.insert_key(key_type, suri, public).map(|_| ())
			})
			.map_err(|e| {
				println!("Error inserting key: {:?}", e);
			})
	);
}
