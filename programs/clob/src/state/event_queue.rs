use anchor_lang::prelude::*;

pub const MAX_EVENTS: usize = 256;

#[account(zero_copy)]
#[derive(InitSpace)]
pub struct EventQueue {
    pub head: u64,                       // Queue head index
    pub tail: u64,                       // Queue tail index
    pub capacity: u64,                   // Queue capacity
    pub events: [FillEvent; MAX_EVENTS], // Events array
}

#[zero_copy]
#[derive(InitSpace)]
#[repr(C)]
pub struct FillEvent {
    pub maker_order_id: u64,
    pub taker_order_id: u64,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
    pub maker_owner: Pubkey,
    pub taker_owner: Pubkey,
    pub market: Pubkey,
    pub maker_side: u8,    // Maker order side (0=Bid, 1=Ask)
    pub _padding: [u8; 7], // Explicit padding to avoid automatic padding
}

impl EventQueue {
    pub fn push_event(&mut self, event: FillEvent) -> Result<()> {
        require!(!self.is_full(), crate::errors::ErrorCode::EventQueueFull);

        self.events[self.tail as usize] = event;
        self.tail = (self.tail + 1) % self.capacity;

        Ok(())
    }

    pub fn pop_event(&mut self) -> Result<FillEvent> {
        require!(!self.is_empty(), crate::errors::ErrorCode::EventQueueEmpty);

        let event = self.events[self.head as usize];
        self.head = (self.head + 1) % self.capacity;

        Ok(event)
    }

    pub fn peek_event(&self) -> Result<FillEvent> {
        require!(!self.is_empty(), crate::errors::ErrorCode::EventQueueEmpty);

        let event = self.events[self.head as usize];
        Ok(event)
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        (self.tail + 1) % self.capacity == self.head
    }

    pub fn len(&self) -> u64 {
        if self.tail >= self.head {
            self.tail - self.head
        } else {
            self.capacity - self.head + self.tail
        }
    }
}
