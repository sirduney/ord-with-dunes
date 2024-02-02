use {
  super::*,
};

pub(crate) struct Rtx<'a>(pub(crate) redb::ReadTransaction<'a>);

impl Rtx<'_> {
  pub(crate) fn height(&self) -> Result<Option<Height>> {
    Ok(
      self
        .0
        .open_table(HEIGHT_TO_BLOCK_HASH)?
        .range(0..)?
        .rev()
        .next()
        .map(|result| {
          result.map(|(height, _hash)| Height(height.value()))
        })
        .transpose()? // Converts Option<Result<T, E>> to Result<Option<T>, E>
    )
  }

  pub(crate) fn block_count(&self) -> Result<u32> {
    Ok(
      self
        .0
        .open_table(HEIGHT_TO_BLOCK_HASH)?
        .range(0..)?
        .rev()
        .next()
        .map(|result| {
          result.map(|(height, _hash)| height.value() + 1)
        })
        .transpose()?  // Converts Option<Result<T, E>> to Result<Option<T>, E> and propagates error if any
        .unwrap_or(0),
    )
  }
}
