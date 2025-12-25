enable wgpu_mesh_shader;

struct TaskPayload {
    colorMask: vec4<f32>,
    visible: bool,
}

var<task_payload> taskPayload: TaskPayload;
var<workgroup> workgroupData: f32;

@task
@payload(taskPayload)
@workgroup_size(1)
fn task() -> @builtin(mesh_task_size) vec3<u32> {
    workgroupData = 1.0;
    taskPayload.colorMask = vec4(1.0, 1.0, 0.0, 1.0);
    taskPayload.visible = true;
    return vec3(1, 1, 1);
}
