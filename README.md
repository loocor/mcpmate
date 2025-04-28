# MCP-Client

MCP-Client 是一个基于 Rust 的 Model Context Protocol (MCP) 客户端和代理服务器实现。

## 项目目标

本项目旨在将原有的 Python 实现（现已存档在 `archive` 目录中）重构为 Rust 实现，以提高性能、可靠性和可维护性。主要功能包括：

1. **MCP 客户端**：用于连接和管理 MCP 服务器
2. **MCP 代理服务器**：将多个 MCP 服务器的工具整合到一起，提供统一的接口

## 技术基础

本项目基于 [Model Context Protocol Rust SDK](https://github.com/modelcontextprotocol/rust-sdk) 进行开发，该 SDK 提供了完整的 MCP 客户端和服务器实现框架。

## 配置文件

项目使用与原 Python 版本相同的 `mcp.json` 配置文件格式，确保兼容性和平滑迁移。

## 示例

`sample` 目录包含了各种 MCP 工具调用的示例配置。
