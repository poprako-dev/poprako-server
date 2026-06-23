# Expected Way to Use

```rust
pub fn make_order<O, A, B>(user_id: &str, manager: &Manager<B>, order_repo: &O, account_repo: &A) -> UsecaseResult<Order>
where
    O: OrderRepository, // OrderRepository should implements Avdvance<CreateOrderCmd> or something like that.
    A: AccountRepository,
    B: Backend,
{
    #[derive(Advance)]
    struct Advance<O, A> {
        #[advance(CreateOrderCmd)]
        order_repo: O,
        #[advance(DrawMoneyCmd)]
        account_repo: A,
    }

    let order = manager.transactional_scoped(
        |context| Advance::from_context(context),
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
