use stm32f4xx_hal::gpio::{Alternate, Output, PushPull, PA2, PB0, PB14, PB7, PC1};

pub type LedGreenPin = PB0<Output<PushPull>>;
pub type LedBluePin = PB7<Output<PushPull>>;
pub type LedRedPin = PB14<Output<PushPull>>;

pub type PhyMdioPin = PA2<Alternate<PushPull, 11>>;
pub type PhyMdcPin = PC1<Alternate<PushPull, 11>>;
