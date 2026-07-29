# Data 层注意事项

## Data 层命名规则

直接对应 Request 的结构体叫做 *Instr，非 Info 投影的直接 Response DTO 叫做 *Val。因为入参是一种指令，而返回值通常只是一种 VO。

直接对应领域 *Info 的结构体，无论是否被 Vec、Option 或其他 Response DTO 包装，都叫做 *InfoView；其他只作为 Response DTO 内部组成部分的结构体叫做 *View。
