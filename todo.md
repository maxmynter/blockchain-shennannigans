# Blockchain Implementation To-Do List

## Weird Stuff
- [ ] Wtf is the maxsize of a mempool. Does that even make sense?

## High Priority
- [ ] Figure out the proper logic when it does mine itself and when does it not do it. E.g. the generate and mempool/generate endpoint -- is that even necessary anymore? 
- [ ] Create background mining/block generation task
  - [ ] Implement as configurable async task
  - [ ] Integrate with mempool for transaction selection

## Medium Priority
- [ ] Develop efficient chain synchronization
  - [ ] Implement block header sync first
  - [ ] Add targeted block retrieval
- [ ] Add transaction broadcasting
  - [ ] Create validation before broadcasting
  - [ ] Implement propagation mechanism
- [ ] Clean up legacy code
  - [ ] Remove single-message implementation
  - [ ] Update all UI components
- [ ] Make the difficulty addaptive (and remove it from persistence)

## Low Priority
- [ ] Dockerize application
  - [ ] Create multi-stage build
  - [ ] Develop docker-compose for testing
- [ ] Implement dynamic block difficulty
  - [ ] Target specific block time
  - [ ] Create difficulty adjustment algorithm
- [ ] Improve mempool transaction selection
  - [ ] Add fee-based priority

## Queue
- [ ] Transform Data Persistence of chain into proper Transaction / Wallet / Account thing
- [ ] Miners should earn when they find a block (or stake) -> What is the best abstraction in the consensus here? 
