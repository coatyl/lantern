import { describe, it, expect } from "vitest";

import { parseVolumePath } from "./volumePath";

describe("parseVolumePath", () => {
  it("splits a POSIX path into name and directory", () => {
    expect(parseVolumePath("/home/ada/bookmarks/chrome.html")).toEqual({
      path: "/home/ada/bookmarks/chrome.html",
      name: "chrome.html",
      directory: "home/ada/bookmarks",
    });
  });

  it("splits a Windows path into name and directory", () => {
    expect(parseVolumePath("C:\\Users\\ada\\Documents\\firefox.html")).toEqual({
      path: "C:\\Users\\ada\\Documents\\firefox.html",
      name: "firefox.html",
      directory: "C:/Users/ada/Documents",
    });
  });

  it("treats a bare filename as a name with no directory", () => {
    expect(parseVolumePath("bookmarks.html")).toEqual({
      path: "bookmarks.html",
      name: "bookmarks.html",
      directory: "",
    });
  });
});
