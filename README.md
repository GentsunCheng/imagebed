# 基于 Rust 和 Actix-web 的简易图床

这是某个课程设计项目的一部分。我同时还负责了[该项目的前端](https://github.com/Eslzzyl/graduate-info-frontend)。

使用了 Rust 知名 Web 框架 Actix-web。

目前主干代码是一晚上用 ChatGPT 仓促糊完的，近期会完善一下。

完善之后我会补上尽量完善的中英文档。

I will add English RAEDME after I complete this project.

## 使用方法 | Usage

### 启动服务

1. 克隆仓库

    ```shell
    git clone git@github.com:Eslzzyl/imagebed.git
    ```

2. 配置 `config.toml`

    你需要先决定是直接把这个服务暴露到外网，还是用反向代理。推荐后者。然后确定本程序的端口号，默认值是7879（关于 config 的默认值，可以看 `./src/config.rs`）。

    - 如果使用反向代理，在你的 Web 服务器中配置，将你希望的域名和访问路径反代到内网的 `localhost:[port]`，然后编辑 `config.toml`文件，根据实际需要配置 `www_root`；`proxy` 写 `true`；`ssl` 根据实际情况写（根据你的反向代理服务器是否配置SSL来确定，`ssl` 配置项目前仅仅决定返回的URL是 `http` 开头还是 `https` 开头）；`host` 写 `localhost`，`port` 写你在反向代理中配置的端口，`local` 写 `true`。
    - 如果不使用反向代理，编辑 `config.toml`文件，根据实际需要配置 `www_root`；`proxy` 写 `false`，`ssl` 写 `false`（本程序目前不支持 SSL）；`host` 写你部署的服务器的 IP 或者域名，`port` 写你决定的端口，`local` 写 `false`。
      - 如果你不使用反向代理，那么这个程序可能很不安全，请你权衡风险。

3. 运行

    ```shell
    cargo run
    ```

    程序本身没多少行代码，但 Actix-web 框架还是比较重的，编译这个程序对机器配置有一定要求。可用内存最好在 500MB 以上。

    如果要长期运行，建议用 Release 构建。

### 使用服务

你可以访问这个服务的 `root`（以默认配置为例，是`http://localhost:7879`）来查看一个简单的导航页。

上传文件：向 `/upload` 发送一个 POST 请求，携带要上传的文件，最好一个请求只传一个文件，我没有测试过传多个文件能不能用，根据我写代码时的感觉，估计是不能的。然后服务在这个请求的响应中会给出文件的直链。于是你就可以保存并使用这个直链了。

文件名是通过将文件内容和处理请求的时间进行 SHA256 哈希，得到的结果从中间截断，作为两个 128 位数字相加，舍去进位，作为 16 进制输出得到的。因此只要一秒内没有传两个相同的文件，就不会出现文件重复的情况（不考虑哈希碰撞）。

## 开发计划 | Plans

- 计划开发一个申请删除功能，对于请求的文件进行删除。
- 计划通过sqlite之类的服务来支持对文件信息的存储，然后就可以用一些算法来淘汰长时间没用过的文件。
- HTML仍然需要改进。

## 警告 | Warning

这个项目写得十分潦草，不适合生产环境部署。如果你需要正规的图床程序，请找找别的！

This project is so crude that it is not suitable for production environment. Please look for other image hosting services if you need.