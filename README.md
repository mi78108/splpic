# splpic
Export Hikvision NAS Store Picture
导出海康威视 网络存储的图片。

* 指定时间范围-年月日时分秒

`splipic -s/home/nas -t/mnt/target -d20191010101010-20191010111111`

or  从-至今

`splipic -s/home/nas -t/mnt/target -d20191010101010`

* 指定时间范围-时间戳

`splipic -s/home/nas -t/mnt/target -u12345678-12345678`

* 指定输出子目录命名(时间)

`splipic -s/home/nas -t/mnt/target -r%Y-%m-%d -u12345678-12345678`

* 指定线程数量(默认 5)

`splipic -n50 -s/home/nas -t/mnt/target -r%Y-%m-%d -u12345678-12345678`

* 日志打印在 标准输出 `stderr` 标准输出中`stdout` 打印导出的文件绝对路径

