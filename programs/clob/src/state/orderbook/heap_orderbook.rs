use super::{
    order::{Fill, Order, Side},
    traits::OrderBook,
};
use crate::errors::ErrorCode;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use std::marker::PhantomData;

const MAX_ORDERS: usize = 1024; // Reduced to fit in Solana's stack limit

/// Heap kind marker traits for order comparison
pub trait Kind: Clone + Default + Copy + 'static {
    /// Compare two orders based on the heap type (max or min)
    fn compare(a: &Order, b: &Order) -> bool;
    const SIDE: Side;
}

/// Max heap - higher price first, then earlier timestamp (Bid side)
#[derive(Clone, Default, Copy)]
pub struct Max;
impl Kind for Max {
    fn compare(a: &Order, b: &Order) -> bool {
        match a.price.cmp(&b.price) {
            std::cmp::Ordering::Greater => true,
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Equal => a.timestamp < b.timestamp,
        }
    }
    const SIDE: Side = Side::Bid;
}

/// Min heap - lower price first, then earlier timestamp (Ask side)
#[derive(Clone, Default, Copy)]
pub struct Min;
impl Kind for Min {
    fn compare(a: &Order, b: &Order) -> bool {
        match a.price.cmp(&b.price) {
            std::cmp::Ordering::Less => true,
            std::cmp::Ordering::Greater => false,
            std::cmp::Ordering::Equal => a.timestamp < b.timestamp,
        }
    }
    const SIDE: Side = Side::Ask;
}

/// Generic fixed-size orderbook implementation
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SimpleOrderBook<K: Kind> {
    data: [Order; MAX_ORDERS],
    len: u32,
    _kind: PhantomData<K>,
}

unsafe impl<K: Kind> Pod for SimpleOrderBook<K> {}
unsafe impl<K: Kind> Zeroable for SimpleOrderBook<K> {}

impl<K: Kind> Default for SimpleOrderBook<K> {
    fn default() -> Self {
        Self {
            data: [Order::default(); MAX_ORDERS],
            len: 0,
            _kind: PhantomData,
        }
    }
}

impl<K: Kind> SimpleOrderBook<K> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn peek(&self) -> Option<&Order> {
        if self.len == 0 {
            None
        } else {
            Some(&self.data[0])
        }
    }

    pub fn push(&mut self, item: Order) -> Result<()> {
        if self.len >= MAX_ORDERS as u32 {
            return Err(error!(ErrorCode::OrderbookFull));
        }

        let index = self.len as usize;
        self.data[index] = item;
        self.len += 1;
        self.bubble_up(index);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<Order> {
        match self.len {
            0 => None,
            1 => {
                self.len = 0;
                Some(self.data[0])
            }
            _ => {
                let last_index = (self.len - 1) as usize;
                let result = self.data[0];
                self.data[0] = self.data[last_index];
                self.len -= 1;
                self.bubble_down(0);
                Some(result)
            }
        }
    }

    pub fn remove<F>(&mut self, predicate: F) -> Option<Order>
    where
        F: Fn(&Order) -> bool,
    {
        let len = self.len as usize;
        let position = (0..len).find(|&i| predicate(&self.data[i]))?;

        match position {
            pos if pos == len - 1 => {
                self.len -= 1;
                Some(self.data[pos])
            }
            0 => {
                let result = self.data[0];
                let last_index = (self.len - 1) as usize;
                self.data[0] = self.data[last_index];
                self.len -= 1;
                self.bubble_down(0);
                Some(result)
            }
            pos => {
                let removed_item = self.data[pos];
                let last_index = (self.len - 1) as usize;
                self.data[pos] = self.data[last_index];
                self.len -= 1;

                if pos > 0
                    && K::compare(
                        &self.data[pos],
                        &self.data[Self::parent_index(pos).unwrap()],
                    )
                {
                    self.bubble_up(pos);
                } else {
                    self.bubble_down(pos);
                }

                Some(removed_item)
            }
        }
    }

    pub fn find<F>(&self, predicate: F) -> Option<&Order>
    where
        F: Fn(&Order) -> bool,
    {
        let len = self.len as usize;
        (0..len).find_map(|i| {
            if predicate(&self.data[i]) {
                Some(&self.data[i])
            } else {
                None
            }
        })
    }

    fn parent_index(index: usize) -> Option<usize> {
        if index == 0 {
            None
        } else {
            Some((index - 1) / 2)
        }
    }

    fn left_child_index(index: usize) -> usize {
        2 * index + 1
    }

    fn right_child_index(index: usize) -> usize {
        2 * index + 2
    }

    fn bubble_up(&mut self, mut index: usize) {
        while let Some(parent_idx) = Self::parent_index(index) {
            if K::compare(&self.data[index], &self.data[parent_idx]) {
                self.data.swap(index, parent_idx);
                index = parent_idx;
            } else {
                break;
            }
        }
    }

    fn bubble_down(&mut self, mut index: usize) {
        let len = self.len as usize;
        loop {
            let mut best = index;
            let left = Self::left_child_index(index);
            let right = Self::right_child_index(index);

            if left < len && K::compare(&self.data[left], &self.data[best]) {
                best = left;
            }

            if right < len && K::compare(&self.data[right], &self.data[best]) {
                best = right;
            }

            if best != index {
                self.data.swap(index, best);
                index = best;
            } else {
                break;
            }
        }
    }
}

// Implement OrderBook trait for the generic SimpleOrderBook
impl<K: Kind> OrderBook for SimpleOrderBook<K> {
    fn insert_order(&mut self, order: Order) -> Result<()> {
        self.push(order)
    }

    fn remove_order(&mut self, order_id: u64) -> Result<Option<Order>> {
        Ok(self.remove(|order| order.order_id == order_id))
    }

    fn get_best_price(&self) -> Option<u64> {
        self.peek().map(|order| order.price)
    }

    fn match_orders(&mut self, incoming_order: &mut Order) -> Result<Vec<Fill>> {
        let mut fills = Vec::new();

        while incoming_order.remaining_quantity > 0 {
            let best_order = match self.peek() {
                Some(order) => *order,
                None => break,
            };

            // Check if orders can match based on the Kind's side
            let can_match = match K::SIDE {
                Side::Bid => {
                    // This is a bid book: incoming ask order matches with bid orders at >= price
                    best_order.price >= incoming_order.price
                }
                Side::Ask => {
                    // This is an ask book: incoming bid order matches with ask orders at <= price
                    best_order.price <= incoming_order.price
                }
            };

            if !can_match {
                break; // No more matching possible
            }

            let mut existing_order = self.pop().unwrap();
            let fill_quantity = existing_order
                .remaining_quantity
                .min(incoming_order.remaining_quantity);

            let fill = Fill {
                maker_order_id: existing_order.order_id,
                taker_order_id: incoming_order.order_id,
                price: existing_order.price, // Use maker price
                quantity: fill_quantity,
            };
            fills.push(fill);

            existing_order.remaining_quantity -= fill_quantity;
            incoming_order.remaining_quantity -= fill_quantity;

            if existing_order.remaining_quantity > 0 {
                self.push(existing_order)?;
            }
        }

        Ok(fills)
    }

    fn get_order(&self, order_id: u64) -> Option<&Order> {
        self.find(|order| order.order_id == order_id)
    }

    fn len(&self) -> usize {
        self.len as usize
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Type aliases for convenience
pub type BidOrderBook = SimpleOrderBook<Max>;
pub type AskOrderBook = SimpleOrderBook<Min>;
