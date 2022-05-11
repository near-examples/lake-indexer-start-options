# lake-indexer-start-options

This is a source code for the tutorial [Extending Lake indexer with start options](https://near-indexers.io/tutorials/lake/lake-start-options). It shows how to implement start options to your indexer built on top of [NEAR Lake Framework](https://github.com/near/near-lake-framework):

- from specified block height (out of the box)
  ```bash
  ./target/release/indexer mainnet from-block 65359506
  ```

- from the latest final block from the network
  ```bash
  ./target/release/indexer mainnet from-latest
  ```

- from the block indexer has indexed the last before it was interrupted
