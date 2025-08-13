use anchor_lang::prelude::*;
use super::{
    order::{Order, Side, Fill},
    traits::OrderBook,
};

// Vec-based implementation for initial version
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, InitSpace)]
pub struct VecOrderBook {
    #[max_len(50)]  // Maximum 50 orders for initial implementation
    pub orders: Vec<Order>,
    pub side: Side, // Bid or Ask
}

impl VecOrderBook {
    pub fn new(side: Side) -> Self {
        Self {
            orders: Vec::new(),
            side,
        }
    }
}

impl OrderBook for VecOrderBook {
    fn insert_order(&mut self, order: Order) -> Result<()> {
        // Find insertion position to maintain price-time priority
        let insert_pos = match self.side {
            Side::Bid => {
                // Bids: highest price first, then earliest time
                self.orders.iter().position(|existing| {
                    existing.price < order.price
                        || (existing.price == order.price && existing.timestamp > order.timestamp)
                })
            }
            Side::Ask => {
                // Asks: lowest price first, then earliest time
                self.orders.iter().position(|existing| {
                    existing.price > order.price
                        || (existing.price == order.price && existing.timestamp > order.timestamp)
                })
            }
        };

        match insert_pos {
            Some(pos) => self.orders.insert(pos, order),
            None => self.orders.push(order),
        }

        Ok(())
    }

    fn remove_order(&mut self, order_id: u64) -> Result<Option<Order>> {
        if let Some(pos) = self.orders.iter().position(|order| order.order_id == order_id) {
            Ok(Some(self.orders.remove(pos)))
        } else {
            Ok(None)
        }
    }

    fn get_best_price(&self) -> Option<u64> {
        self.orders.first().map(|order| order.price)
    }

    fn match_orders(&mut self, incoming_order: &mut Order) -> Result<Vec<Fill>> {
        let mut fills = Vec::new();
        let mut orders_to_remove = Vec::new();

        for (index, existing_order) in self.orders.iter_mut().enumerate() {
            // Check if orders can match
            let can_match = match self.side {
                Side::Bid => {
                    // incoming ask order matches with bid orders at >= price
                    existing_order.price >= incoming_order.price
                }
                Side::Ask => {
                    // incoming bid order matches with ask orders at <= price
                    existing_order.price <= incoming_order.price
                }
            };

            if !can_match {
                break; // Orders are sorted, no more matches possible
            }

            // Calculate fill quantity
            let fill_quantity = existing_order.remaining_quantity.min(incoming_order.remaining_quantity);

            // Create fill record
            let fill = Fill {
                maker_order_id: existing_order.order_id,
                taker_order_id: incoming_order.order_id,
                price: existing_order.price, // Use maker price
                quantity: fill_quantity,
            };
            fills.push(fill);

            // Update quantities
            existing_order.remaining_quantity -= fill_quantity;
            incoming_order.remaining_quantity -= fill_quantity;

            // Mark fully filled orders for removal
            if existing_order.remaining_quantity == 0 {
                orders_to_remove.push(index);
            }

            // If incoming order is fully filled, stop matching
            if incoming_order.remaining_quantity == 0 {
                break;
            }
        }

        // Remove fully filled orders (in reverse order to maintain indices)
        for &index in orders_to_remove.iter().rev() {
            self.orders.remove(index);
        }

        Ok(fills)
    }

    fn get_order(&self, order_id: u64) -> Option<&Order> {
        self.orders.iter().find(|order| order.order_id == order_id)
    }

    fn len(&self) -> usize {
        self.orders.len()
    }

    fn is_empty(&self) -> bool {
        self.orders.is_empty()
    }
}