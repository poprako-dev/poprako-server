# Data 层注意事项

## Data 层命名规则

直接对应 Request、Response 的结构体叫做 \*Instr, \*Val。因为入参是一种指令，而返回值通常只是一种 VO。

而不对应 Val，但是包含于 Val 内的次级结构叫做 \*View。
