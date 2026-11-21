fn total(order: &Order) -> u64 {
    let subtotal = price(
        order.items,
        order.discount,
        order.region,
    );
    subtotal + shipping(order)
}
