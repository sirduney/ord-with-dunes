use {super::*, boilerplate::Boilerplate};

pub(crate) use {
  block::{BlockHtml, BlockHashAndConfirmations},
  home::HomeHtml,
  iframe::Iframe,
  input::InputHtml,
  inscription::InscriptionHtml,
  inscriptions::InscriptionsHtml,
  output::OutputHtml,
  output::OutputJson,
  page_config::PageConfig,
  preview::{
    PreviewAudioHtml, PreviewImageHtml, PreviewPdfHtml, PreviewTextHtml, PreviewUnknownHtml,
    PreviewVideoHtml,
  },
  range::RangeHtml,
  rare::RareTxt,
  dune::DuneEntryJson,
  dune::DuneHtml,
  utxo::Utxo,
  dune::{DuneAddressJson, DuneJson, DuneOutputJson, DuneBalance, DuneOutput},
  dune_balances::DuneBalancesHtml,
  dunes::DunesHtml,
  sat::SatHtml,
  transaction::TransactionHtml,
};

mod block;
mod home;
mod iframe;
mod input;
mod inscription;
mod inscriptions;
mod output;
mod preview;
mod range;
mod rare;
mod dune;
mod dune_balances;
mod dunes;
mod sat;
mod transaction;
mod utxo;

#[derive(Boilerplate)]
pub(crate) struct PageHtml<T: PageContent> {
  content: T,
  config: Arc<PageConfig>,
}

impl<T> PageHtml<T>
where
  T: PageContent,
{
  pub(crate) fn new(content: T, config: Arc<PageConfig>) -> Self {
    Self { content, config }
  }

  fn og_image(&self) -> String {
    if let Some(domain) = &self.config.domain {
      format!("https://{domain}/static/favicon.png")
    } else {
      "https://ordinals.com/static/favicon.png".into()
    }
  }

  fn superscript(&self) -> String {
    if self.config.chain == Chain::Mainnet {
      "alpha".into()
    } else {
      self.config.chain.to_string()
    }
  }
}

pub(crate) trait PageContent: Display + 'static {
  fn title(&self) -> String;

  fn page(self, page_config: Arc<PageConfig>) -> PageHtml<Self>
  where
    Self: Sized,
  {
    PageHtml::new(self, page_config)
  }

  fn preview_image_url(&self) -> Option<Trusted<String>> {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct Foo;

  impl Display for Foo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
      write!(f, "<h1>Foo</h1>")
    }
  }

  impl PageContent for Foo {
    fn title(&self) -> String {
      "Foo".to_string()
    }
  }

  #[test]
  fn page() {
    assert_regex_match!(
      Foo.page(Arc::new(PageConfig {
        chain: Chain::Mainnet,
        domain: Some("signet.ordinals.com".into()),
        index_sats: true,
      }),),
      r"<!doctype html>
<html lang=en>
  <head>
    <meta charset=utf-8>
    <meta name=format-detection content='telephone=no'>
    <meta name=viewport content='width=device-width,initial-scale=1.0'>
    <meta property=og:title content='Foo'>
    <meta property=og:image content='https://signet.ordinals.com/static/favicon.png'>
    <meta property=twitter:card content=summary>
    <title>Foo</title>
    <link rel=alternate href=/feed.xml type=application/rss\+xml title='Inscription RSS Feed'>
    <link rel=stylesheet href=/static/index.css>
    <link rel=stylesheet href=/static/modern-normalize.css>
    <script src=/static/index.js defer></script>
  </head>
  <body>
  <header>
    <nav>
      <a href=/>Dunes<sup>alpha</sup></a>
      .*
      <a href=/rare.txt>rare.txt</a>
      <form action=/search method=get>
        <input type=text .*>
        <input type=submit value=Search>
      </form>
    </nav>
  </header>
  <main>
<h1>Foo</h1>
  </main>
  </body>
</html>
"
    );
  }

  #[test]
  fn page_mainnet() {
    assert_regex_match!(
      Foo.page(Arc::new(PageConfig {
        chain: Chain::Mainnet,
        domain: None,
        index_sats: true,
      }),),
      r".*<nav>\s*<a href=/>Dunes<sup>alpha</sup></a>.*"
    );
  }

  #[test]
  fn page_no_sat_index() {
    assert_regex_match!(
      Foo.page(Arc::new(PageConfig {
        chain: Chain::Mainnet,
        domain: None,
        index_sats: false,
      }),),
      r".*<nav>\s*<a href=/>Dunes<sup>alpha</sup></a>.*\s*<form action=/search.*",
    );
  }

  #[test]
  fn page_signet() {
    assert_regex_match!(
      Foo.page(Arc::new(PageConfig {
        chain: Chain::Signet,
        domain: None,
        index_sats: true,
      }),),
      r".*<nav>\s*<a href=/>Dunes<sup>signet</sup></a>.*"
    );
  }
}
