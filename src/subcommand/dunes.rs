use super::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Output {
  pub dunes: BTreeMap<Dune, DuneInfo>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DuneInfo {
  pub burned: u128,
  pub deadline: Option<u32>,
  pub divisibility: u8,
  pub end: Option<u32>,
  pub etching: Txid,
  pub height: u32,
  pub id: DuneId,
  pub index: u16,
  pub limit: Option<u128>,
  pub number: u64,
  pub dune: Dune,
  pub spacers: u32,
  pub supply: u128,
  pub symbol: Option<char>,
  pub timestamp: DateTime<Utc>,
}

pub(crate) fn run(options: Options) -> SubcommandResult {
  let index = Index::open(&options)?;

  ensure!(
    index.has_dune_index(),
    "`ord dunes` requires index created with `--index-dunes` flag",
  );

  index.update()?;

  Ok(Box::new(Output {
    dunes: index
      .dunes()?
      .into_iter()
      .map(
        |(
          id,
          DuneEntry {
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
          },
        )| {
          (
            dune,
            DuneInfo {
              burned,
              deadline,
              divisibility,
              etching,
              height: id.height,
              id,
              index: id.index,
              end,
              limit,
              number,
              timestamp: crate::timestamp(timestamp),
              dune,
              spacers,
              supply,
              symbol,
            },
          )
        },
      )
      .collect::<BTreeMap<Dune, DuneInfo>>(),
  }))
}
