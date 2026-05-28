mgr-run:
    diesel migration run

mgr-revert:
    diesel migration revert

mgr-create name:
    diesel migration generate {{name}}

# This command is only used for creating database.
mgr-setup:
    diesel database setup

