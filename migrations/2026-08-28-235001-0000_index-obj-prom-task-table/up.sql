CREATE INDEX IF NOT EXISTS "i_obj_prom_task_poll"
    ON "t_obj_prom_task" (
        "f_status",
        "f_visible_at",
        "f_created_at",
        "f_id"
    );

CREATE INDEX IF NOT EXISTS "i_obj_prom_task_stuck"
    ON "t_obj_prom_task" (
        "f_status",
        "f_updated_at",
        "f_lease"
    );
