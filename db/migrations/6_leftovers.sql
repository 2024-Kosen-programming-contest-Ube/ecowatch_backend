ALTER TABLE day_status ADD "leftovers" INT AFTER "attend";
CREATE UNIQUE INDEX day_status_unique_index ON day_status("class_id", "date");