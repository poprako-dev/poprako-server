# Use-case Notes

## Cascading deletion

`delete_cascade` releases a resource subtree. Database cascade constraints do
not remove objects from the image pool, so the use case coordinates explicit
prom deletion after the transaction succeeds.

Each resource deletes only its immediate children. A child owns cleanup of its
own descendants.

## `incl` and `with`

`incl` embeds directly related records into a result. Dotted options expand
their parent path, such as `chapter.comic.workset.team`.

`with` attaches a derived relation rather than an ordinary included record. A
comic's pinned chapter is an example: it is selected by a special condition,
not by a one-to-one foreign-key relation.

## Naming

Public use cases use short operation names such as `get_info`, `list_infos`,
`create`, `update_info`, `save_infos`, and `delete`. Put filtering and other
variable conditions in a request/spec value instead of encoding them in the
name.

Repository steps may name a storage-specific selection, lock, or exclusion
when that detail belongs to the persistence operation.

## Use of `data` structs must be qualified with `*_data::`

Use `user_data::UserInfoView` instead of raw `UserInfoView`, for example.
