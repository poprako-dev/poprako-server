#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$project_root"

for rust_file in $(find src poprako-util/src poprako-swagger/src -type f -name '*.rs' | LC_ALL=C sort); do
    max_lines=600

    case "$rust_file" in
        src/complex/chapter.rs) max_lines=671 ;;
        src/complex/chapter_port/import.rs) max_lines=770 ;;
        src/part_impl/prom/mock_impl.rs) max_lines=643 ;;
        src/part_impl/repo/mock_impl/chapter/orchestra.rs) max_lines=670 ;;
        src/part_impl/repo/mock_impl/comic.rs) max_lines=718 ;;
        src/part_impl/repo/rdb_impl/comic/step_impl.rs) max_lines=674 ;;
        src/part_impl/repo/rdb_impl/team.rs) max_lines=626 ;;
        src/part_impl/repo/rdb_impl/user.rs) max_lines=656 ;;
        src/usecase/assignment_invitation/tests.rs) max_lines=605 ;;
        src/usecase/chapter/tests.rs) max_lines=633 ;;
        src/usecase/comic.rs) max_lines=655 ;;
        src/usecase/page.rs) max_lines=609 ;;
        src/usecase/team.rs) max_lines=652 ;;
        src/usecase/user.rs) max_lines=626 ;;
        src/usecase/user/tests.rs) max_lines=634 ;;
        src/value/chapter.rs) max_lines=657 ;;
    esac

    line_count=$(wc -l < "$rust_file")

    if [ "$line_count" -gt "$max_lines" ]; then
        echo "$rust_file has $line_count lines; maximum is $max_lines" >&2
        exit 1
    fi
done
