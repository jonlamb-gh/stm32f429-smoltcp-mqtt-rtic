use stm32_eth::{RingEntry, RxDescriptor, TxDescriptor};

const RX_DESC_RING_COUNT: usize = 8;
const TX_DESC_RING_COUNT: usize = 4;

const RX_DESC_INIT: RingEntry<RxDescriptor> = RingEntry::<RxDescriptor>::new();
const TX_DESC_INIT: RingEntry<TxDescriptor> = RingEntry::<TxDescriptor>::new();

pub struct EthStorage {
    pub rx_ring: [RingEntry<RxDescriptor>; RX_DESC_RING_COUNT],
    pub tx_ring: [RingEntry<TxDescriptor>; TX_DESC_RING_COUNT],
}

impl EthStorage {
    pub const fn new() -> Self {
        Self {
            rx_ring: [RX_DESC_INIT; RX_DESC_RING_COUNT],
            tx_ring: [TX_DESC_INIT; TX_DESC_RING_COUNT],
        }
    }
}
