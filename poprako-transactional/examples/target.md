# Expected Way to Use

```rust
pub fn make_order<OR, AR, B>(user_id: &str, manager: &Manager<B>, order_repo: &OR, account_repo: &AR) -> UsecaseResult<Order>
where
    OR: OrderRepository, // OrderRepository should implements Avdvance<CreateOrderCmd> or something like that.
    AR: AccountRepository,
    B: Backend,
{
    #[derive(Advance)]
    struct Advance<OR, AR> {
        #[advance(CreateOrderCmd)]
        order_repo: OR,
        #[advance(DrawMoneyCmd)]
        account_repo: AR,
    }

    let order = manager.transactional_scoped(
        |handle| Advance::from_handle(handle),
        async move |proxy| {
            proxy.run(DrawMoneyCmd::new(user_id, 100)).await?;
            let order = proxy.run(CreateOrderCmd::new(user_id)).await?;

            Ok(order)
        }
        .boxed()
    )
    .await?;

    Ok(())
}
```
