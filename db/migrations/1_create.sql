CREATE TABLE school("id" TEXT, "school_name" TEXT);
CREATE TABLE classroom("id" TEXT, "school_id" TEXT, "grade" INTEGER, "name" TEXT, "password_hash" TEXT, UNIQUE("school_id", "grade", "name"));
CREATE TABLE day_status("date" TEXT, "class_id" TEXT, "point" TEXT, "attend" INTEGER);
CREATE TABLE sensor_log("class_id" TEXT, "time" TEXT, "values" TEXT);
CREATE TABLE teacher("id" TEXT, "class_id" TEXT, "email" TEXT, "password_hash" TEXT);
CREATE TABLE checklist("class_id" TEXT, "student_id" TEXT, list TEXT);

CREATE TABLE class_token("token" TEXT, "class_id" TEXT, "expired_time" TEXT);
CREATE TABLE teacher_token("token" TEXT, "teacher_id" TEXT, "expired_time" TEXT);
CREATE TABLE student_token("token" TEXT, "student_id" TEXT, "class_id" TEXT, "expired_time" TEXT);