CREATE TABLE school("id" TEXT NOT NULL PRIMARY KEY, "name" TEXT NOT NULL);
CREATE TABLE classroom("id" TEXT NOT NULL PRIMARY KEY, "school_id" TEXT NOT NULL, "grade" INTEGER NOT NULL, "name" TEXT NOT NULL, "password_hash" TEXT NOT NULL, UNIQUE("school_id", "grade", "name"));
CREATE TABLE day_status("class_id" TEXT NOT NULL, "point" INTEGER NOT NULL, "attend" INTEGER, "date" TEXT NOT NULL);
CREATE TABLE sensor_log("class_id" TEXT, "time" TEXT, "values" TEXT);
CREATE TABLE teacher("id" TEXT, "class_id" TEXT, "email" TEXT, "password_hash" TEXT);
CREATE TABLE checklist("class_id" TEXT, "student_id" TEXT, list TEXT);

CREATE TABLE class_token("token" TEXT NOT NULL PRIMARY KEY, "class_id" TEXT NOT NULL);
CREATE TABLE teacher_token("token" TEXT NOT NULL PRIMARY KEY, "teacher_id" TEXT NOT NULL);
CREATE TABLE student_token("token" TEXT NOT NULL PRIMARY KEY, "student_id" INTEGER NOT NULL, "class_id" TEXT NOT NULL);