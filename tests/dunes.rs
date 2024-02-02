use {super::*, ord::subcommand::dunes::Output};

#[test]
fn flag_is_required() {
  let rpc_server = test_bitcoincore_rpc::builder()
    .network(Network::Regtest)
    .build();

  CommandBuilder::new("--regtest dunes")
    .rpc_server(&rpc_server)
    .expected_exit_code(1)
    .expected_stderr("error: `ord dunes` requires index created with `--index-dunes` flag\n")
    .run_and_extract_stdout();
}

#[test]
fn no_dunes() {
  let rpc_server = test_bitcoincore_rpc::builder()
    .network(Network::Regtest)
    .build();

  assert_eq!(
    CommandBuilder::new("--index-dunes --regtest dunes")
      .rpc_server(&rpc_server)
      .run_and_deserialize_output::<Output>(),
    Output {
      dunes: BTreeMap::new(),
    }
  );
}

#[test]
fn one_dune() {
  let rpc_server = test_bitcoincore_rpc::builder()
    .network(Network::Regtest)
    .build();

  create_wallet(&rpc_server);

  let etch = etch(&rpc_server, Dune(DUNE));

  assert_eq!(
    CommandBuilder::new("--index-dunes --regtest dunes")
      .rpc_server(&rpc_server)
      .run_and_deserialize_output::<Output>(),
    Output {
      dunes: vec![(
        Dune(DUNE),
        DuneInfo {
          burned: 0,
          divisibility: 0,
          end: None,
          etching: etch.transaction,
          height: 2,
          id: DuneId {
            height: 2,
            index: 1
          },
          index: 1,
          limit: None,
          number: 0,
          dune: Dune(DUNE),
          supply: 1000,
          symbol: Some('¢'),
          timestamp: ord::timestamp(2),
        }
      )]
      .into_iter()
      .collect(),
    }
  );
}

#[test]
fn two_dunes() {
  let rpc_server = test_bitcoincore_rpc::builder()
    .network(Network::Regtest)
    .build();

  create_wallet(&rpc_server);

  let a = etch(&rpc_server, Dune(DUNE));
  let b = etch(&rpc_server, Dune(DUNE + 1));

  assert_eq!(
    CommandBuilder::new("--index-dunes --regtest dunes")
      .rpc_server(&rpc_server)
      .run_and_deserialize_output::<Output>(),
    Output {
      dunes: vec![
        (
          Dune(DUNE),
          DuneInfo {
            burned: 0,
            divisibility: 0,
            end: None,
            etching: a.transaction,
            height: 2,
            id: DuneId {
              height: 2,
              index: 1
            },
            index: 1,
            limit: None,
            number: 0,
            dune: Dune(DUNE),
            supply: 1000,
            symbol: Some('¢'),
            timestamp: ord::timestamp(2),
          }
        ),
        (
          Dune(DUNE + 1),
          DuneInfo {
            burned: 0,
            divisibility: 0,
            end: None,
            etching: b.transaction,
            height: 4,
            id: DuneId {
              height: 4,
              index: 1
            },
            index: 1,
            limit: None,
            number: 1,
            dune: Dune(DUNE + 1),
            supply: 1000,
            symbol: Some('¢'),
            timestamp: ord::timestamp(4),
          }
        )
      ]
      .into_iter()
      .collect(),
    }
  );
}
