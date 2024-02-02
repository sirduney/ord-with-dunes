use crate::sat::Sat;
use crate::sat_point::SatPoint;
use super::*;

pub(super) trait Entry: Sized {
  type Value;

  fn load(value: Self::Value) -> Self;

  fn store(self) -> Self::Value;
}

pub(super) type BlockHashValue = [u8; 32];

impl Entry for BlockHash {
  type Value = BlockHashValue;

  fn load(value: Self::Value) -> Self {
    BlockHash::from_inner(value)
  }

  fn store(self) -> Self::Value {
    self.into_inner()
  }
}

pub(super) type TxidValue = [u8; 32];

impl Entry for Txid {
  type Value = TxidValue;

  fn load(value: Self::Value) -> Self {
    Txid::from_inner(value)
  }

  fn store(self) -> Self::Value {
    self.into_inner()
  }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) struct DuneEntry {
  pub(crate) burned: u128,
  pub(crate) deadline: Option<u32>,
  pub(crate) divisibility: u8,
  pub(crate) end: Option<u32>,
  pub(crate) etching: Txid,
  pub(crate) limit: Option<u128>,
  pub(crate) number: u64,
  pub(crate) dune: Dune,
  pub(crate) spacers: u32,
  pub(crate) supply: u128,
  pub(crate) symbol: Option<char>,
  pub(crate) timestamp: u32,
}

pub(super) type DuneEntryValue = (
  u128,         // burned
  Option<u32>,  // deadline
  u8,           // divisibility
  u32,          // end
  (u128, u128), // etching
  u128,         // limit
  u64,          // number
  u128,         // dune
  u32,          // spacers
  u128,         // supply
  u32,          // symbol
  u32,          // timestamp
);

impl DuneEntry {
  pub(crate) fn spaced_dune(&self) -> SpacedDune {
    SpacedDune {
      dune: self.dune,
      spacers: self.spacers,
    }
  }
}

impl Default for DuneEntry {
  fn default() -> Self {
    Self {
      burned: 0,
      deadline: None,
      divisibility: 0,
      end: None,
      etching: Txid::all_zeros(),
      limit: None,
      number: 0,
      dune: Dune(0),
      spacers: 0,
      supply: 0,
      symbol: None,
      timestamp: 0,
    }
  }
}

/*pub(super) type TxidValue = [u8; 32];

impl Entry for Txid {
  type Value = TxidValue;

  fn load(value: Self::Value) -> Self {
    Txid::from_byte_array(value)
  }

  fn store(self) -> Self::Value {
    Txid::to_byte_array(self)
  }
}*/

impl Entry for DuneEntry {
  type Value = DuneEntryValue;

  fn load(
    (
      burned,
      deadline,
      divisibility,
      end,
      etching,
      limit,
      number,
      dune,
      spacers,
      supply,
      symbol,
      timestamp,
    ): DuneEntryValue,) -> Self {
    Self {
      burned,
      deadline,
      divisibility,
      end: (end != u32::max_value()).then_some(end),
      etching: {
        let low = etching.0.to_le_bytes();
        let high = etching.1.to_le_bytes();
        let bytes: Vec<u8> = [low, high].concat();
        Txid::from_slice(bytes.as_slice()).unwrap_or(Txid::all_zeros())
      },
      limit: (limit != u128::max_value()).then_some(limit),
      number,
      dune: Dune(dune),
      spacers,
      supply,
      symbol: char::from_u32(symbol),
      timestamp,
    }
  }

  fn store(self) -> Self::Value {
    (
      self.burned,
      self.deadline,
      self.divisibility,
      self.end.unwrap_or(u32::max_value()),
      {
        let bytes_vec = self.etching.to_vec();
        let bytes: [u8; 32] = match bytes_vec.len() {
          32 => {
            let mut array = [0; 32];
            array.copy_from_slice(&bytes_vec);
            array
          },
          _ => panic!("Vector length is not 32"),
        };
        (
          u128::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
          ]),
          u128::from_le_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
            bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31],
          ]),
        )
      },
      self.limit.unwrap_or(u128::max_value()),
      self.number,
      self.dune.0,
      self.spacers,
      self.supply,
      self.symbol.map(u32::from).unwrap_or(u32::max_value()),
      self.timestamp,
    )
  }
}

pub(super) type DuneIdValue = (u32, u16);

impl Entry for DuneId {
  type Value = DuneIdValue;

  fn load((height, index): Self::Value) -> Self {
    Self { height, index }
  }

  fn store(self) -> Self::Value {
    (self.height, self.index)
  }
}

pub(crate) struct InscriptionEntry {
  pub(crate) fee: u64,
  pub(crate) height: u32,
  pub(crate) inscription_number: u64,
  pub(crate) sat: Option<Sat>,
  pub(crate) sequence_number: u64,
  pub(crate) timestamp: u32,
}

pub(crate) type InscriptionEntryValue = (
  u64,         // fee
  u32,         // height
  u64,         // inscription number
  Option<u64>,         // sat
  u64,         // sequence number
  u32,         // timestamp
);

impl Entry for InscriptionEntry {
  type Value = InscriptionEntryValue;

  fn load(
    (fee, height, inscription_number, sat, sequence_number, timestamp): InscriptionEntryValue,
  ) -> Self {
    Self {
      fee,
      height,
      inscription_number,
      sat: sat.map(Sat),
      sequence_number,
      timestamp,
    }
  }

  fn store(self) -> Self::Value {
    (
      self.fee,
      self.height,
      self.inscription_number,
      self.sat.map(Sat::n),
      self.sequence_number,
      self.timestamp,
    )
  }
}

pub(super) type InscriptionIdValue = [u8; 36];

impl Entry for InscriptionId {
  type Value = InscriptionIdValue;

  fn load(value: Self::Value) -> Self {
    let (txid, index) = value.split_at(32);
    Self {
      txid: Txid::from_inner(txid.try_into().unwrap()),
      index: u32::from_be_bytes(index.try_into().unwrap()),
    }
  }

  fn store(self) -> Self::Value {
    let mut value = [0; 36];
    let (txid, index) = value.split_at_mut(32);
    txid.copy_from_slice(self.txid.as_inner());
    index.copy_from_slice(&self.index.to_be_bytes());
    value
  }
}

pub(super) type OutPointValue = [u8; 36];

impl Entry for OutPoint {
  type Value = OutPointValue;

  fn load(value: Self::Value) -> Self {
    Decodable::consensus_decode(&mut io::Cursor::new(value)).unwrap()
  }

  fn store(self) -> Self::Value {
    let mut value = [0; 36];
    self.consensus_encode(&mut value.as_mut_slice()).unwrap();
    value
  }
}

pub(super) type SatPointValue = [u8; 44];

impl Entry for SatPoint {
  type Value = SatPointValue;

  fn load(value: Self::Value) -> Self {
    Decodable::consensus_decode(&mut io::Cursor::new(value)).unwrap()
  }

  fn store(self) -> Self::Value {
    let mut value = [0; 44];
    self.consensus_encode(&mut value.as_mut_slice()).unwrap();
    value
  }
}

pub(super) type SatRange = (u64, u64);

impl Entry for SatRange {
  type Value = [u8; 11];

  fn load([b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10]: Self::Value) -> Self {
    let raw_base = u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, 0]);

    // 51 bit base
    let base = raw_base & ((1 << 51) - 1);

    let raw_delta = u64::from_le_bytes([b6, b7, b8, b9, b10, 0, 0, 0]);

    // 33 bit delta
    let delta = raw_delta >> 3;

    (base, base + delta)
  }

  fn store(self) -> Self::Value {
    let base = self.0;
    let delta = self.1 - self.0;
    let n = u128::from(base) | u128::from(delta) << 51;
    n.to_le_bytes()[0..11].try_into().unwrap()
  }
}
