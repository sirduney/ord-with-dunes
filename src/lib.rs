#![allow(
  clippy::too_many_arguments,
  clippy::type_complexity,
  clippy::result_large_err
)]
#![deny(
  clippy::cast_lossless,
  clippy::cast_possible_truncation,
  clippy::cast_possible_wrap,
  clippy::cast_sign_loss
)]

use {
  self::{
    arguments::Arguments,
    blocktime::Blocktime,
    config::Config,
    decimal::Decimal,
    deserialize_from_str::DeserializeFromStr,
    epoch::Epoch,
    height::Height,
    index::{Index, List, DuneEntry, MintEntry},
    inscription::Inscription,
    inscription_id::InscriptionId,
    media::Media,
    options::Options,
    outgoing::Outgoing,
    representation::Representation,
    dunes::{Etching, Pile, SpacedDune},
    sat::Sat,
    subcommand::Subcommand,
    tally::Tally,
  },
  anyhow::{anyhow, bail, ensure, Context, Error},
  bip39::Mnemonic,
  bitcoin::{
    consensus::{self, Decodable, Encodable},
    hash_types::BlockHash,
    hashes::Hash,
    blockdata::opcodes,
    blockdata::script::{self, Instruction},
    Address, Amount, Block, Network, OutPoint, Script, Sequence, Transaction, TxIn, TxOut, Txid,
    Witness,
  },
  bitcoincore_rpc::{Client, RpcApi},
  chain::Chain,
  chrono::{DateTime, TimeZone, Utc},
  clap::{ArgGroup, Parser},
  derive_more::{Display, FromStr},
  html_escaper::{Escape, Trusted},
  lazy_static::lazy_static,
  regex::Regex,
  serde::{Deserialize, Deserializer, Serialize, Serializer},
  std::{
    cmp,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    env,
    ffi::OsString,
    fmt::{self, Display, Formatter},
    fs::{self, File},
    io,
    net::{TcpListener, ToSocketAddrs},
    ops::{Add, AddAssign, Sub},
    path::{Path, PathBuf},
    process::{self, Command},
    str::FromStr,
    sync::{
      atomic::{self, AtomicBool, AtomicU64},
      Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime},
  },
  sysinfo::System,
  tempfile::TempDir,
  tokio::{runtime::Runtime, task},
};
use crate::sat_point::SatPoint;

pub use self::{
  fee_rate::FeeRate, object::Object, rarity::Rarity,
  dunes::{Edict, Dune, DuneId, Dunestone},
  subcommand::wallet::transaction_builder::{Target, TransactionBuilder},
};

#[cfg(test)]
#[macro_use]
mod test;

#[cfg(test)]
use self::test::*;

macro_rules! tprintln {
    ($($arg:tt)*) => {

      if cfg!(test) {
        eprint!("==> ");
        eprintln!($($arg)*);
      }
    };
}

mod arguments;
mod blocktime;
mod chain;
mod config;
mod decimal;
mod deserialize_from_str;
mod epoch;
mod fee_rate;
mod height;
mod index;

mod decimal_sat;
mod inscription;
mod inscription_id;
mod media;
mod object;
mod options;
mod outgoing;
mod page_config;
mod rarity;
mod representation;
mod dunes;
mod sat;
mod sat_point;
pub mod subcommand;
mod tally;
mod templates;
mod wallet;

type Result<T = (), E = Error> = std::result::Result<T, E>;

const SUBSIDY_HALVING_INTERVAL_10X: u32 =
  bitcoin::blockdata::constants::SUBSIDY_HALVING_INTERVAL * 10;

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
static INTERRUPTS: AtomicU64 = AtomicU64::new(0);
static LISTENERS: Mutex<Vec<axum_server::Handle>> = Mutex::new(Vec::new());

const TARGET_POSTAGE: Amount = Amount::from_sat(10_000);

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn fund_raw_transaction(
  client: &Client,
  fee_rate: FeeRate,
  unfunded_transaction: &Transaction,
) -> Result<Vec<u8>> {
  Ok(
    client
        .fund_raw_transaction(
          unfunded_transaction,
          Some(&bitcoincore_rpc::json::FundRawTransactionOptions {
            // NB. This is `fundrawtransaction`'s `feeRate`, which is fee per kvB
            // and *not* fee per vB. So, we multiply the fee rate given by the user
            // by 1000.
            fee_rate: Some(Amount::from_sat((fee_rate.n() * 1000.0).ceil() as u64)),
            ..Default::default()
          }),
          Some(false),
        )?
        .hex,
  )
}

fn integration_test() -> bool {
  env::var_os("ORD_INTEGRATION_TEST")
    .map(|value| value.len() > 0)
    .unwrap_or(false)
}

pub fn timestamp(seconds: u32) -> DateTime<Utc> {
  Utc.timestamp_opt(seconds.into(), 0).unwrap()
}

const INTERRUPT_LIMIT: u64 = 5;

pub fn main() {
  env_logger::init();

  ctrlc::set_handler(move || {
    LISTENERS
      .lock()
      .unwrap()
      .iter()
      .for_each(|handle| handle.graceful_shutdown(Some(Duration::from_millis(100))));

    println!("Detected Ctrl-C, attempting to shut down ord gracefully. Press Ctrl-C {INTERRUPT_LIMIT} times to force shutdown.");

    let interrupts = INTERRUPTS.fetch_add(1, atomic::Ordering::Relaxed);

    if interrupts > INTERRUPT_LIMIT {
      process::exit(1);
    }
  })
  .expect("Error setting ctrl-c handler");

  if let Err(err) = Arguments::parse().run() {
    eprintln!("error: {err}");
    err
      .chain()
      .skip(1)
      .for_each(|cause| eprintln!("because: {cause}"));
    if env::var_os("RUST_BACKTRACE")
      .map(|val| val == "1")
      .unwrap_or_default()
    {
      eprintln!("{}", err.backtrace());
    }
    process::exit(1);
  }
}
