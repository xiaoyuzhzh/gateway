## 本地运行
直接配置环境变量 CONFIG_PATH = 实际配置文件位置  
配置参考配置文件 config_demo.yaml

## docker
**打包镜像**  
本地运行 docker build -t gateway .  

**运行镜像**：  
docker run --name gateway -v 宿主机配置文件路径:/app/config/config.yaml -d 
--network host gateway
