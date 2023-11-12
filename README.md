# 基于 Rust 和 Actix-web 的简易图床

这是某个课程设计项目的一部分，起因是我需要方便地存储头像信息，又不想在前后端之间发送文件并在后端数据库中保存图片。我同时还负责了[该项目的前端](https://github.com/Eslzzyl/graduate-info-frontend)。

使用了 Rust 知名 Web 框架 Actix-web。

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

    如果要长期运行，建议用 Release 构建：
    ```shell
    cargo run --release
    ```

### 使用服务

- 你可以访问这个服务的 `root`（以默认配置为例，是`http://localhost:7879`）来查看一个简单的导航页。该页面包含了文件上传和删除的功能。我希望尽量保持这个页面的简单性，因此不会添加太多额外的样式。
- 上传文件：向 `/upload` 发送一个 POST 请求，`Content-Type` 为 `multipart/form-data`，携带要上传的文件，一个请求只能传一个文件，如果传了多个，则服务器只会接受第一个。然后服务在这个请求的响应中会给出文件的直链。于是你就可以保存并使用这个直链了。
  - 如果你在配置文件中启用了 token 功能，那么在文件之前还要带上一个额外的 `token` 字段。注意 `token` 是明文传输的，这个功能只是为了限制第三方上传有害的文件，因此不要把 token 视为密码。token 只是一个简单的口令。
  - 文件名是通过将文件内容和处理请求的时间进行 SHA256 哈希，得到的结果从中间截断，作为两个 128 位数字相加，舍去进位，作为 16 进制输出得到的。因此只要一秒内没有传两个相同的文件，就不会出现文件重复的情况（不考虑哈希碰撞）。
- 删除文件：向 `/delete` 发送一个 POST 请求，请求体是一个满足如下格式的 JSON：

    ```json
    {
        "file": "example.jpg"
    }
    ```

    一次只能删除一个文件。这种设计意味着文件名就是你对该文件所有权的唯一证明，只要获知文件名，就可以删除它。

    无论上传时是否要求提供 token，删除时都无需 token。

## 配置文件详解

配置文件的位置是 `config/config.toml` 。

|配置项|类型|描述|
|:-:|:-:|---|
|`www_root`|`&str`|用来存放 `file` 目录的目录。在之前的版本，`index.html` 等服务自带的页面也存放在这里，但现在它们固定存放在 `./www` 下了。之后预计会修改这块的逻辑。|
|`proxy`|`bool`|是否使用反向代理。决定了动态生成请求 URL 时是否会带上端口号。如果为 `true`，则不会带上端口号；如果为 `false`，则会带上端口号。|
|`ssl`|`bool`|是否使用 SSL 连接。决定了动态生成请求 URL 时的协议是 `http` 还是 `https`。|
|`host`|`&str`|主机名。当工作在本地时，可以填写为 `localhost`；工作在公网时，可以填公网 IP 或者域名。|
|`port`|`u16`|监听端口号，默认值为7879|
|`local`|`bool`|是否工作在本地。如果为 `true`，则监听 IP 为 `0.0.0.0` ；如果为 `false`，则监听 IP 为 `127.0.0.1`；推荐的操作是开启反向代理，并在此处设为 `false`。|
|`max_file_size`|`usize`|允许上传的最大文件大小，单位为 MB。|
|`use_token`|`bool`|上传时是否要求提供口令。|
|`token`|`String`|上传时的口令，仅当 `use_token` 为 `true` 时才生效。|

## 代码示例 | Example

使用 [Axios](https://www.axios-http.cn/) 的示例（TypeScript）：
```typescript
import axios from 'axios';

// 是否使用 token 验证功能
const use_token: boolean = false;

// 假设你已经通过<input>之类的标签获得了一个文件，名为 file
const formdata = new FormData();
if (use_token) {
    formData.append("token", token);
}
formData.append('file', file);  // 总是添加 file

// 使用 Axios 发送 POST 请求
axios.post('UPLOAD', formData, {    // 把 UPLOAD 替换为请求 URL
    headers: {
        'Content-Type': 'multipart/form-data',
    }
}).then(response => {
    console.log("文件上传成功，链接为：", response.data);
}).catch(error => {
    if (use_token && error.response.data == "Incorrect token!") {
        console.log("token 不正确，文件上传失败");
    } else {
        console.error("文件上传失败：", error.message);
    }
});
```

## 开发计划 | Plans

- 计划通过 sqlite 之类的服务来支持对文件信息的存储，然后就可以用一些算法来淘汰长时间没用过的文件。
- HTML 样式改进。
- 后端支持交互式操作，支持重载配置文件。
- 用模板引擎重写 index 部分

## 警告 | Warning

这个项目写得十分潦草，不适合生产环境部署。如果你需要正规的图床程序，请找找别的！

This project is so crude that it is not suitable for production environment. Please look for other image hosting services if you need.