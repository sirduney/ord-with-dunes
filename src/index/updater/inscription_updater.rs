use redb::MultimapTable;
use super::*;
use crate::inscription::ParsedInscription;
use crate::sat::Sat;
use crate::sat_point::SatPoint;

pub(super) struct Flotsam {
  inscription_id: InscriptionId,
  offset: u64,
  origin: Origin,
}

enum Origin {
  New(u64),
  Old(SatPoint),
}

pub(super) struct InscriptionUpdater<'a, 'db, 'tx> {
  flotsam: Vec<Flotsam>,
  height: u32,
  id_to_satpoint: &'a mut Table<'db, 'tx, &'static InscriptionIdValue, &'static SatPointValue>,
  id_to_txids: &'a mut Table<'db, 'tx, &'static InscriptionIdValue, &'static [u8]>,
  txid_to_tx: &'a mut Table<'db, 'tx, &'static [u8], &'static [u8]>,
  partial_txid_to_txids: &'a mut Table<'db, 'tx, &'static [u8], &'static [u8]>,
  value_receiver: &'a mut Receiver<u64>,
  transaction_buffer: Vec<u8>,
  transaction_id_to_transaction: &'a mut Table<'db, 'tx, &'static TxidValue, &'static [u8]>,
  id_to_entry: &'a mut Table<'db, 'tx, &'static InscriptionIdValue, InscriptionEntryValue>,
  lost_sats: u64,
  next_number: u64,
  number_to_id: &'a mut Table<'db, 'tx, u64, &'static InscriptionIdValue>,
  outpoint_to_value: &'a mut Table<'db, 'tx, &'static OutPointValue, u64>,
  address_to_outpoint: &'a mut MultimapTable<'db, 'tx, &'static [u8], &'static OutPointValue>,
  reward: u64,
  sat_to_inscription_id: &'a mut Table<'db, 'tx, u64, &'static InscriptionIdValue>,
  satpoint_to_id: &'a mut Table<'db, 'tx, &'static SatPointValue, &'static InscriptionIdValue>,
  timestamp: u32,
  value_cache: &'a mut HashMap<OutPoint, OutPointMapValue>,
}

impl<'a, 'db, 'tx> InscriptionUpdater<'a, 'db, 'tx> {
  pub(super) fn new(
    height: u32,
    id_to_satpoint: &'a mut Table<'db, 'tx, &'static InscriptionIdValue, &'static SatPointValue>,
    id_to_txids: &'a mut Table<'db, 'tx, &'static InscriptionIdValue, &'static [u8]>,
    txid_to_tx: &'a mut Table<'db, 'tx, &'static [u8], &'static [u8]>,
    partial_txid_to_txids: &'a mut Table<'db, 'tx, &'static [u8], &'static [u8]>,
    value_receiver: &'a mut Receiver<u64>,
    transaction_buffer: Vec<u8>,
    transaction_id_to_transaction: &'a mut Table<'db, 'tx, &'static TxidValue, &'static [u8]>,
    id_to_entry: &'a mut Table<'db, 'tx, &'static InscriptionIdValue, InscriptionEntryValue>,
    lost_sats: u64,
    number_to_id: &'a mut Table<'db, 'tx, u64, &'static InscriptionIdValue>,
    outpoint_to_value: &'a mut Table<'db, 'tx, &'static OutPointValue, u64>,
    address_to_outpoint: &'a mut MultimapTable<'db, 'tx, &'static [u8], &'static OutPointValue>,
    sat_to_inscription_id: &'a mut Table<'db, 'tx, u64, &'static InscriptionIdValue>,
    satpoint_to_id: &'a mut Table<'db, 'tx, &'static SatPointValue, &'static InscriptionIdValue>,
    timestamp: u32,
    value_cache: &'a mut HashMap<OutPoint, OutPointMapValue>,
  ) -> Result<Self> {
    let next_number = number_to_id
        .iter()?
        .rev()
        .map(|result| {
          result.map(|(number, _id)| number.value() + 1)
        })
        .next()
        .transpose()?
        .unwrap_or(0);

    Ok(Self {
      flotsam: Vec::new(),
      height,
      id_to_satpoint,
      id_to_txids,
      txid_to_tx,
      partial_txid_to_txids,
      value_receiver,
      transaction_buffer,
      transaction_id_to_transaction,
      id_to_entry,
      lost_sats,
      next_number,
      number_to_id,
      outpoint_to_value,
      address_to_outpoint,
      reward: Height(height).subsidy(),
      sat_to_inscription_id,
      satpoint_to_id,
      timestamp,
      value_cache,
    })
  }

  pub(super) fn index_transaction_inscriptions(
    &mut self,
    tx: &Transaction,
    txid: Txid,
    input_sat_ranges: Option<&VecDeque<(u64, u64)>>,
  ) -> Result<u64> {
    let mut inscriptions = Vec::new();

    tx.consensus_encode(&mut self.transaction_buffer)
        .expect("in-memory writers don't error");
    self
        .transaction_id_to_transaction
        .insert(&txid.store(), self.transaction_buffer.as_slice())?;

    self.transaction_buffer.clear();

    let mut input_value = 0;
    for tx_in in &tx.input {
      if tx_in.previous_output.is_null() {
        input_value += Height(self.height).subsidy();
      } else {
        let result: Result<(), _> = (|| {
          for result in Index::inscriptions_on_output(self.satpoint_to_id, tx_in.previous_output)? {
            let (old_satpoint, inscription_id) = result?;
            inscriptions.push(Flotsam {
              offset: input_value + old_satpoint.offset,
              inscription_id,
              origin: Origin::Old(old_satpoint),
            });
          }
          Ok(())
        })();

        if let Err(e) = result {
          // Propagate the error e
          return Err(e);
        }

        input_value += if let Some(map) = self.value_cache.remove(&tx_in.previous_output) {
          map.0
        } else if let Some(map) = self
            .outpoint_to_value
            .remove(&tx_in.previous_output.store())?
        {
          if let Some(transaction) = self
              .transaction_id_to_transaction
              .get(&tx_in.previous_output.txid.store())? {
            let tx: Transaction = consensus::encode::deserialize(transaction.value())?;
            let output = tx.output[tx_in.previous_output.vout as usize].clone();
            if let Some(address_from_script) = Chain::Mainnet
                .address_from_script(&output.script_pubkey)
                .ok() {
              self.address_to_outpoint.remove(address_from_script.to_string().as_bytes(), &tx_in.previous_output.store())?;
            }
          }
          map.value()
        } else {
          self.value_receiver.blocking_recv().ok_or_else(|| {
            anyhow!(
              "failed to get transaction for {}",
              tx_in.previous_output.txid
            )
          })?
        }
      }
    }

    if inscriptions.iter().all(|flotsam| flotsam.offset != 0) {
      let previous_txid = tx.input[0].previous_output.txid;
      let previous_txid_bytes: [u8; 32] = previous_txid.into_inner();
      let mut txids_vec = vec![];

      let txs = match self
          .partial_txid_to_txids
          .get(&previous_txid_bytes.as_slice())?
      {
        Some(partial_txids) => {
          let txids = partial_txids.value();
          let mut txs = vec![];
          txids_vec = txids.to_vec();
          for i in 0..txids.len() / 32 {
            let txid = &txids[i * 32..i * 32 + 32];
            let tx_result = self.txid_to_tx.get(txid)?;
            let tx_result = tx_result.unwrap();
            let tx_buf = tx_result.value();
            let mut cursor = std::io::Cursor::new(tx_buf);
            let tx = bitcoin::Transaction::consensus_decode(&mut cursor)?;
            txs.push(tx);
          }
          txs.push(tx.clone());
          txs
        }
        None => {
          vec![tx.clone()]
        }
      };

      match Inscription::from_transactions(txs) {
        ParsedInscription::None => { }

        ParsedInscription::Partial => {
          let mut txid_vec = txid.into_inner().to_vec();
          txids_vec.append(&mut txid_vec);

          self
              .partial_txid_to_txids
              .remove(&previous_txid_bytes.as_slice())?;
          self
              .partial_txid_to_txids
              .insert(&txid.into_inner().as_slice(), txids_vec.as_slice())?;

          let mut tx_buf = vec![];
          tx.consensus_encode(&mut tx_buf)?;
          self
              .txid_to_tx
              .insert(&txid.into_inner().as_slice(), tx_buf.as_slice())?;
        }

        ParsedInscription::Complete(_inscription) => {
          self
              .partial_txid_to_txids
              .remove(&previous_txid_bytes.as_slice())?;

          let mut tx_buf = vec![];
          tx.consensus_encode(&mut tx_buf)?;
          self
              .txid_to_tx
              .insert(&txid.into_inner().as_slice(), tx_buf.as_slice())?;

          let mut txid_vec = txid.into_inner().to_vec();
          txids_vec.append(&mut txid_vec);

          let mut inscription_id = [0_u8; 36];
          unsafe {
            std::ptr::copy_nonoverlapping(txids_vec.as_ptr(), inscription_id.as_mut_ptr(), 32)
          }
          self
              .id_to_txids
              .insert(&inscription_id, txids_vec.as_slice())?;

          let og_inscription_id = InscriptionId {
            txid: Txid::from_slice(&txids_vec[0..32]).unwrap(),
            index: 0
          };

          inscriptions.push(Flotsam {
            inscription_id: og_inscription_id,
            offset: 0,
            origin: Origin::New(
              input_value - tx.output.iter().map(|txout| txout.value).sum::<u64>(),
            ),
          });
        }
      }
    };

    let is_coinbase = tx
        .input
        .first()
        .map(|tx_in| tx_in.previous_output.is_null())
        .unwrap_or_default();

    if is_coinbase {
      inscriptions.append(&mut self.flotsam);
    }

    inscriptions.sort_by_key(|flotsam| flotsam.offset);
    let mut inscriptions = inscriptions.into_iter().peekable();

    let mut output_value = 0;
    for (vout, tx_out) in tx.output.iter().enumerate() {
      let end = output_value + tx_out.value;

      while let Some(flotsam) = inscriptions.peek() {
        if flotsam.offset >= end {
          break;
        }

        let new_satpoint = SatPoint {
          outpoint: OutPoint {
            txid,
            vout: vout.try_into().unwrap(),
          },
          offset: flotsam.offset - output_value,
        };

        self.update_inscription_location(
          input_sat_ranges,
          inscriptions.next().unwrap(),
          new_satpoint,
        )?;
      }

      output_value = end;

      let address_from_script = Chain::Mainnet.address_from_script(&tx_out.clone().script_pubkey);

      let address = if address_from_script.is_err() {
        [0u8; 34]
      } else {
        address_from_script
            .unwrap()
            .to_string()
            .as_bytes()
            .try_into()
            .unwrap_or_else(|_| panic!("Failed to convert slice to array")) // Handle conversion failure
      };

      self.value_cache.insert(
        OutPoint {
          vout: vout.try_into().unwrap(),
          txid,
        },
        (
          tx_out.clone().value,
          address,
        )
      );
    }

    if is_coinbase {
      for flotsam in inscriptions {
        let new_satpoint = SatPoint {
          outpoint: OutPoint::null(),
          offset: self.lost_sats + flotsam.offset - output_value,
        };
        self.update_inscription_location(input_sat_ranges, flotsam, new_satpoint)?;
      }

      Ok(self.reward - output_value)
    } else {
      self.flotsam.extend(inscriptions.map(|flotsam| Flotsam {
        offset: self.reward + flotsam.offset - output_value,
        ..flotsam
      }));
      self.reward += input_value - output_value;
      Ok(0)
    }
  }

  fn update_inscription_location(
    &mut self,
    input_sat_ranges: Option<&VecDeque<(u64, u64)>>,
    flotsam: Flotsam,
    new_satpoint: SatPoint,
  ) -> Result {
    let inscription_id = flotsam.inscription_id.store();

    match flotsam.origin {
      Origin::Old(old_satpoint) => {
        self.satpoint_to_id.remove(&old_satpoint.store())?;
      }
      Origin::New(fee) => {
        self
            .number_to_id
            .insert(&self.next_number, &inscription_id)?;

        let mut sat = None;
        if let Some(input_sat_ranges) = input_sat_ranges {
          let mut offset = 0;
          for (start, end) in input_sat_ranges {
            let size = end - start;
            if offset + size > flotsam.offset {
              let n = start + flotsam.offset - offset;
              self.sat_to_inscription_id.insert(&n, &inscription_id)?;
              sat = Some(Sat(n));
              break;
            }
            offset += size;
          }
        }

        self.id_to_entry.insert(
          &inscription_id,
          &InscriptionEntry {
            fee,
            height: self.height,
            inscription_number: self.next_number,
            sat,
            sequence_number: 0,
            timestamp: self.timestamp,
          }
              .store(),
        )?;

        self.next_number += 1;
      }
    }

    let new_satpoint = new_satpoint.store();

    self.satpoint_to_id.insert(&new_satpoint, &inscription_id)?;
    self.id_to_satpoint.insert(&inscription_id, &new_satpoint)?;

    Ok(())
  }
}
