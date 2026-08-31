# TO FIX

- [ ] ObjPool 应该是在 obj-dept 中定义的一个 trait，然后 R2 只是它的一个实现，这不应该是 poprako-server 的一个 part，因为它根本不会被 usecase 层使用！根本就不该在 poprako-server 中有 ObjRemote 这种垃圾 trait。
- [ ] ActorHarn 是什么垃圾命名？它就叫 ActorDesc，所有负责 eventloop 的叫 *Actor，控制 Actor 的控制句柄叫做 *ActorDesc。
- [ ] ActorInner 命名也是错误的。
- [ ] 怎么还有 get_upload_slot 这种命名？？这个函数是他妈无副作用幂等的吗你就用 get？我不是都写了 gen？
- [ ] ObjError 是什么玩意？你的结构体难道叫 Obj？我怎么不知道？我写的不是 ObjDept 吗？
- [ ] 后面来了新的存储需求，你怎么扩展？比如 Font。你为什么不把样板代码用宏简化。
- [ ] 到底是怎么保证类型安全的？必须保证没有的表和列编译期就报错。现在看起来根本不是这样。
- [ ] 我的 phantom data 的字段除了 \_m 以外的名字允许你写了吗？？？
- [ ] obj-dept 的 rdb_impl 是啥啊？？？他妈逼的一个完全不类型安全的莫名其妙的 trait。我说得类型安全是，diesel 可以在编译器立刻发现你没有构建好对应的表和列的安全。也就是说在不使用任何其他 diesel 的 trick 的情况下，直接可以被 diesel 生成的 schema 安全推断的。

- [ ] obj-dept 的 model 里面嵌套 obj 什么几把意思？？？？？？他妈逼的不按功能分 submod 是他妈逼什么意思？？？
- [ ] ObjPoolSpec 是个什么垃圾命名啊？它他妈是创建还是搜索 ObjPool 啊你他妈逼这么命名？
- [ ] settle 这个名字不可接受，必须包括类型和变量在哪全部改名。
- [ ] 这个 settle 是什么几把操作啊？？还有这么还是有莫名其妙的 Bind 啊？？？
- [ ] 为什么 HybNucl 的定义污染到了 harn.rs 里？？？
- [ ] ObjActor 为什么他妈逼的会以数组存储啊？？？你是弱智吗？？？
- [ ] 严禁有包含 start 名字的任何方法和结构体！！！他妈逼的 actor 当然是 new 函数里就直接他妈逼启动后台 task 啊？？？你他妈逼的手动启动个屁。
- [ ] 我什么时候允许 oper 类结构体可以有自己的构造函数了？？？
- [ ] RdbObjDept 它是他妈逼的 rdb 的 impl 吗？？他妈它不是还有 R2 吗？？？你他妈逼的不叫 NormObjDept？？？
- [ ] 你不会吧 obj! 改成 objs_def! 吗？？？写那么多遍 obj 宏你不觉得有问题吗？？？？
- [ ] 他妈逼除了 use 里为什么还有他妈逼的 crate:: 的调用方式啊？？？？？
- [ ] 到处莫名其妙、毫无用处的 must use 有个屁用？？？

- [ ] 为什么 part 里对应的部分不是叫 obj_dept 而是叫 obj？
