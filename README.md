<div align="center">
  <img src="assets/uniproc-preview.png" alt="Uniproc Icon"/>

**A system monitor for Windows 11 and WSL.**

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![UI](https://img.shields.io/badge/UI-WinUI%203-blue.svg)](https://learn.microsoft.com/windows/apps/winui/winui3/)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-blue.svg)]()
</div>

---

## What is it?

**Uniproc** is a system monitor for Windows 11 built with **Rust** and **WinUI 3**. It shows processes, services and
machine metrics from Windows, WSL and Docker in one place, instead of one tool per environment.

## Motivation

* **WSL transparency.** Stop treating WSL as a `vmmem` black box. Uniproc shows the resource consumption of every
  individual Linux process — [asked for in 2021](https://github.com/microsoft/WSL/issues/6881) and still not resolved
  officially.
* **Appearance.** Tools like Process Hacker are capable but look their age. Uniproc follows Fluent Design, so a
  monitoring tool does not have to feel like it was built in 2003.
