# Model 实现规范

## Model 的定位

Model 是整个程序处理当中的核心对象，它理论上必然持有所有数据的 **所有权**。
它在 usecase 层被创建，所有被 usecase 层调用的函数等只获得它的引用。

Model 是高度结构化的，几乎不会像 data 层结构体一样扁平。它必须高度自描述，以让自己可以字眼看出哪些字段是可组合的，哪些字段又是逻辑冲突的。

## Write Model 与 Read Model 之间的关系

通常来说，Read Model 会包含更多信息，也更本质。所以出现可能的循环引用问题时，优先让 Write 依赖 Read 而非反之。

## Model 命名方式

通常来说，对应 CRUD 的 Write Model 命名分别为：\*Entry, \*Repl(Put 语义), \*Patch(Patch 语义)（Delete 通常来说只需要 id 或者 ids，不列入）。其中每个操作可以根据业务含义调整后缀（比如 Edit），或者添加次级前缀（比如 \*InfoRepl, \*CredsRepl）之类的。

## Model 分层方式

Model 分为读、写两大内容，其中写通常仅作为参数，所以不在分 submod，全部列在 write mod 下；读 Model 则既可能是入参，也可能是返回值，因此需要分开为 spec 和 proj。
