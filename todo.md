Dockerize application

Next steps:
- Mempool cleanup after Block addition
- Build transaction broadcasting (of validated transaction)
- Background task that automatically solves for blocks (like the polling mechanism)
- Remove old single message/ single block implementation in favour of mempooled versions.

- create more efficient sync (not whole chain, but block headers, only getting missing blocks, etc.)
- Blockchain storage persistence
- Dynamic block difficulty (?)
- Broadcasting of generated blocks.
- Consensus implementation for selection of transactions from mempool
