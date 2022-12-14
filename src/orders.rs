

// place_order
pub fn place_order<C: Client + Debug>(
    client: &mut C,
    order_id: i32,
    contract: &Contract,
    order: i32,
) -> Result<()>> {
}

// cancel_order
pub fn cancel_order<C: Client + Debug>(
    client: &mut C,
    order_id: i32,
) -> Result<()>> {
}
