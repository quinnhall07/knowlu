// DRAFT, not wired (task-3 model study, PR #61): a deterministic ceiling on a task's effort_hours,
// read from the title alone, for the cases the pinned model would not keep under the prompt's own
// ceilings (Midterm 8, Final Exam 24, a 12-page paper 60 on ba6a177's run). It only ever lowers.
import { assertEquals } from "@std/assert";
import { effortCeiling, guardEffort } from "./judge_effort_guard.ts";

Deno.test("an exam, midterm, quiz or test is capped at 6", () => {
  for (const title of ["Midterm Exam", "Final Exam", "Exam 2", "Unit 3 Test", "Chapter 7 Reading Quiz"]) {
    assertEquals(effortCeiling(title), 6, title);
  }
  assertEquals(guardEffort(24, "Final Exam"), 6);
});

Deno.test("a stated page count caps at 2 hours a page, ahead of every other rule", () => {
  assertEquals(effortCeiling("Research Paper (12 pages)"), 24);
  assertEquals(effortCeiling("Essay 2 (5 pages)"), 10);
  assertEquals(effortCeiling("Term paper, 10-12 pages"), 24);
  assertEquals(effortCeiling("Thesis chapter (30 pages)"), 40);
  assertEquals(guardEffort(60, "Research Paper (12 pages)"), 24);
});

Deno.test("a named large project or paper keeps the full clamp; its milestone, proposal or draft is capped at 8", () => {
  assertEquals(effortCeiling("Final Project"), 40);
  assertEquals(effortCeiling("Research Paper"), 40);
  assertEquals(effortCeiling("Capstone"), 40);
  assertEquals(effortCeiling("Group Project Milestone 2: Prototype"), 8);
  assertEquals(effortCeiling("Final Project Proposal"), 8);
  assertEquals(effortCeiling("Paper draft"), 8);
});

Deno.test("anything else is capped at 8, and the guard never raises an estimate", () => {
  assertEquals(effortCeiling("Assignment 3"), 8);
  assertEquals(effortCeiling("Case Study Report"), 8);
  assertEquals(guardEffort(2, "Final Exam"), 2);
  assertEquals(guardEffort(0.5, "Assignment 3"), 0.5);
  assertEquals(guardEffort(12, "Lab Report 5"), 8);
});

Deno.test("a project or paper word beats an exam word: a final project is not an exam", () => {
  assertEquals(effortCeiling("Final Project"), 40);
  assertEquals(effortCeiling("Test Plan for the Group Project"), 40);
});
