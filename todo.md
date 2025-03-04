Dockerize application

Next steps:
- add mempool for pending messages and transactions
- create more efficient sync (not whole chain, but block headers, only getting missing blocks, etc.)
- Blockchain storage persistence
- Dynamic block difficulty (?)
- Remove old single message/ single block implementation in favour of mempooled versions.
- Build transaction broadcasting (of validated transaction)
- Broadcasting of generated blocks.
- Mempool cleanup after Block addition
- Background task that automatically solves for blocks (like the polling mechanism)
- Consensus implementation for selection of transactions from mempool
