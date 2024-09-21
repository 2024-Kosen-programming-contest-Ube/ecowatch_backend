DROP TABLE checklist;
CREATE TABLE checklist("class_id" TEXT NOT NULL, "student_id" TEXT NOT NULL, "list" TEXT NOT NULL, "date" TEXT NOT NULL, UNIQUE("class_id", "student_id", "date"));