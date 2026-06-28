# 注意点

## delete_cascade 设计准则

delete cascade 专门用于处理在资源依赖树上，父资源回收自己的子资源子树。

这是因为数据库虽然有 ON DELETE CASCADE，但是图池当中存储的图片不会自动级联删除，所以需要搭配 prom 做显式删除。

最后是，一个父资源只需要显式调用自己的 **一级直属子资源** 的 delete cascade。而子资源怎么处理自己的派生子资源是封装在其内的。

## with 与 incl 的机制

在资源树上，很多父资源与子资源之间都有一些一对一的关系。

比如向上的单对应关系：assignment -> chapter -> comic -> workset -> team。这就是一个典型的从 leaf 到 root 的向上溯源过程，全程可以唯一确定父资源。
所以，这类可以直接对应一套 (id, resource) （比如 (comic_id, comic_info)）的，附带查询机制被称之为 incl。

而有一些父资源，可以唯一地确定一类特殊的子资源（注意，不是代表子资源只有一个，而是特殊条件下只有一个）。比如 comic 的 pinned chapter 是唯一的。而这种附带查询被称之为 with。
