# 本地启动

1. 本地启动一个 postgres 实例.
2. 拷贝 `.env.sample` 为 `.env`，修改数据库配置.
3. 执行 `diesel migration run --locked-schema`.
4. 编译并启动 `./target/debug/server`.
