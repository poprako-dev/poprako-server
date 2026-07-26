# Complex 层注意事项

## Unit Compress 要求

Delete 全部在 Save 前。
Delete ID 不重复。
Save ID 不重复。
同一个 ID 不会同时存在 Delete 和 Save。
剩余 Delete 必须指向 base_ids。
before_id 不能引用自身。
before_id 必须是已有 ID或本批次 Save ID。
before_id 不能指向最终被 Delete 的 ID。
