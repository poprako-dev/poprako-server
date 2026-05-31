mgr-run:
    diesel migration run

mgr-rev:
    diesel migration revert

mgr-add name:
    diesel migration generate {{name}}

# This command is only used for creating database.
mgr-setup:
    diesel database setup

