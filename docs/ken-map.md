```rust
pub trait KeyMap {
    type Dom;

    type Img;

    fn forward(&self, x: &Self::Dom) -> Img;

    fn reverse(&self, y: &Self:;Img) -> ObjDeptRest<Dom>;
}
```
