<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue';

const props = defineProps<{
    show: boolean;
}>();

// SmoothMove 缓动算法
class SmoothMove {
    public m_v0 = 0;
    public m_v1 = 0;
    public m_x0 = 0;
    public m_x1 = 0;
    public m_dx = 0;
    public m_start_time = 0;
    public m_total_duration = 0;
    public m_v = 0;

    public newEndPosition(x: number, time: number): void {
        this.m_x0 = this.m_x1;
        this.m_v0 = this.m_v1;
        this.m_dx = x - this.m_x0;
        this.m_start_time = time;
    }

    public setStartPosition(x: number, time: number): void {
        this.m_x0 = x;
        this.m_x1 = x;
        this.m_v0 = 0;
        this.m_dx = 0;
        this.m_start_time = time;
    }

    public setTotalDuration(time: number): void {
        this.m_total_duration = time;
    }

    public update(time: number): void {
        const t = time - this.m_start_time;
        if (t >= this.m_total_duration) {
            this.m_x1 = this.m_x0 + this.m_dx;
            this.m_v1 = 0;
            return;
        }
        const _1_div_exp_2 = 1.0 / Math.exp(2.0);
        this.m_x1 = this.m_x0 + t * (this.m_v0 + (this.m_v0 - this.m_dx / this.m_total_duration) / (1 - _1_div_exp_2) * (Math.exp(-2 * t / this.m_total_duration) - 1));
        this.m_v1 = ((this.m_total_duration - 2 * t) * (this.m_v0 * this.m_total_duration - this.m_dx) * Math.exp(-2 * t / this.m_total_duration) + this.m_total_duration * this.m_dx - this.m_v0 * this.m_total_duration * this.m_total_duration * _1_div_exp_2) / (this.m_total_duration * this.m_total_duration * (1 - _1_div_exp_2));
        this.m_v = this.m_v1;
    }

    public updateSin(time: number): void {
        let t = time - this.m_start_time;
        if (t >= this.m_total_duration) {
            this.m_x1 = this.m_x0 + this.m_dx;
            this.m_v1 = 0;
            return;
        }
        const t0 = t;
        t = t / this.m_total_duration * Math.PI / 2;
        t = Math.sin(t) * this.m_total_duration;
        const dt0_dt = Math.cos(t0 / this.m_total_duration * Math.PI / 2) * Math.PI / 2;

        const _1_div_exp_2 = 1.0 / Math.exp(2.0);
        this.m_x1 = this.m_x0 + t * (this.m_v0 + (this.m_v0 - this.m_dx / this.m_total_duration) / (1 - _1_div_exp_2) * (Math.exp(-2 * t / this.m_total_duration) - 1));
        this.m_v1 = ((this.m_total_duration - 2 * t) * (this.m_v0 * this.m_total_duration - this.m_dx) * Math.exp(-2 * t / this.m_total_duration) + this.m_total_duration * this.m_dx - this.m_v0 * this.m_total_duration * this.m_total_duration * _1_div_exp_2) / (this.m_total_duration * this.m_total_duration * (1 - _1_div_exp_2));
        this.m_v = dt0_dt * this.m_v1;
    }

    public clamp(min: number, max: number, time: number): void {
        if (this.x() < min) {
            this.newEndPosition(min, time);
        }
        if (this.x() > max) {
            this.newEndPosition(max, time);
        }
    }

    public v(): number {
        return this.m_v;
    }

    public x(): number {
        return this.m_x1;
    }
}

// 粒子类型定义
interface Particle {
    // 位置与显示属性
    x: number;
    y: number;
    size: number;

    // 颜色控制
    baseColor: string;         // 基础颜色，无透明度
    rgbValues: number[];       // RGB值缓存，避免重复计算

    // 物理属性
    angle: number;             // 当前角度
    initialAngle: number;      // 初始角度
    rotationSpeed: number;     // 旋转速度
    radiusOffset: number;      // 呼吸效果偏移量

    // 半径控制
    initialRadius: number;     // 起始半径（屏幕外）
    rotatingRadius: number;    // 旋转半径（中心位置）
    finalRadius: number;       // 最终半径（扩散到屏幕外）

    // 缓动控制器 - 修改为明确的类型声明
    radiusController: SmoothMove;   // 控制半径
    opacityController: SmoothMove;  // 控制透明度
    breathController: SmoothMove;   // 控制呼吸效果振幅
    rotateController: SmoothMove;  // 控制旋转速度
}

// 阶段配置
const phaseDurations = {
    entering: 1,      // 从屏幕外进入中心
    rotating: 3.0,      // 在中心旋转
    exiting: 0.5,       // 扩散到屏幕外
    transitionTime: 1 // 阶段之间的过渡时间
};

// 状态管理
const visible = ref(props.show);
const particles = ref<Particle[]>([]);
const animationFrameId = ref<number | null>(null);
const startTime = ref<number>(0);
const lastFrameTime = ref<number>(0);
const phaseStartTime = ref<number>(0);
const phase = ref<'entering' | 'rotating' | 'exiting'>('entering');
const isExiting = ref(false);

// 监听 show 属性变化
watch(() => props.show, (newValue) => {
    if (!newValue) {
        // 处理退出动画
        isExiting.value = true;
        phase.value = 'exiting';
        const currentTime = performance.now() / 1000;
        phaseStartTime.value = currentTime;

        // 为所有粒子设置退出参数
        particles.value.forEach(particle => {
            prepareParticleForExit(particle, currentTime);
        });

        setTimeout(() => {
            visible.value = false;
            if (animationFrameId.value !== null) {
                cancelAnimationFrame(animationFrameId.value);
                animationFrameId.value = null;
            }
        }, phaseDurations.exiting * 1000 + 200); // 额外时间确保动画完成
    } else {
        visible.value = true;
        isExiting.value = false;
        phase.value = 'entering';
        if (animationFrameId.value === null) {
            startAnimation();
        }
    }
});

function mix(min: number, max: number, blend: number): number {
    return min + (max - min) * blend;
}

function getR(initialRadius: number, rotatingRadius: number, blend: number): number {
    return mix(rotatingRadius, initialRadius, -Math.log(Math.max(Math.min(blend, 1), 1e-6)));
}

// 按照建议的初始化方式配置粒子
function initParticles() {
    const colors = ['#4f46e5', '#6366f1', '#8b5cf6', '#a855f7']; // 紫色系
    const count = 12;
    const newParticles: Particle[] = [];
    const currentTime = performance.now() / 1000;

    for (let i = 0; i < count; i++) {
        const angle = (i / count) * Math.PI * 2;
        const initialRadius = 200;   // 起始于屏幕外
        const rotatingRadius = 40;   // 中心位置半径
        const finalRadius = 300;     // 扩散结束半径
        const baseColor = colors[Math.floor(Math.random() * colors.length)];

        // 创建RGB值缓存
        const r = parseInt(baseColor.slice(1, 3), 16);
        const g = parseInt(baseColor.slice(3, 5), 16);
        const b = parseInt(baseColor.slice(5, 7), 16);

        // 创建控制器实例 - 确保每个控制器都是 SmoothMove 实例
        const radiusController = new SmoothMove();
        radiusController.setStartPosition(0, currentTime);
        radiusController.newEndPosition(1, currentTime);
        radiusController.setTotalDuration(phaseDurations.entering + Math.random() * phaseDurations.transitionTime);

        const opacityController = new SmoothMove();
        opacityController.setStartPosition(0, currentTime);
        opacityController.newEndPosition(1, currentTime);
        opacityController.setTotalDuration(phaseDurations.entering + Math.random() * phaseDurations.transitionTime);

        const breathController = new SmoothMove();
        breathController.setStartPosition(0, currentTime);
        breathController.newEndPosition(0, currentTime);
        breathController.setTotalDuration(phaseDurations.transitionTime + Math.random() * phaseDurations.transitionTime);

        const rotateController = new SmoothMove();
        rotateController.setStartPosition(0, currentTime);
        rotateController.newEndPosition(1, currentTime);
        rotateController.setTotalDuration(phaseDurations.transitionTime);


        // 创建粒子对象
        const particle: Particle = {
            x: Math.cos(angle) * getR(initialRadius, rotatingRadius, 0),
            y: Math.sin(angle) * getR(initialRadius, rotatingRadius, 0),
            size: 4 + Math.random() * 3,
            baseColor,
            rgbValues: [r, g, b],
            angle,
            initialAngle: angle,
            rotationSpeed: 0.3 + Math.random() * 0.3,
            radiusOffset: Math.random() * Math.PI * 2,
            initialRadius,
            rotatingRadius,
            finalRadius,
            radiusController,
            opacityController,
            breathController,
            rotateController
        };

        newParticles.push(particle);
    }

    particles.value = newParticles;
}

// 准备粒子进入旋转阶段
function prepareParticleForRotation(particle: Particle, currentTime: number) {
    // 确保半径稳定在旋转值

    // 设置呼吸效果
    particle.breathController.newEndPosition(8, currentTime); // 设置呼吸振幅

    // 保持完全不透明
    particle.opacityController.newEndPosition(1, currentTime);
}

// 准备粒子进入退出阶段
function prepareParticleForExit(particle: Particle, currentTime: number) {
    // 设置从当前位置到最终扩散位置
    particle.radiusController.newEndPosition(0, currentTime);

    // 关闭呼吸效果
    particle.breathController.newEndPosition(0, currentTime);

    // 设置从当前不透明度到完全透明
    particle.opacityController.newEndPosition(0, currentTime);

    // 设置旋转速度为0，保持当前角度
    particle.rotateController.newEndPosition(0, currentTime);
}

// 动画循环
function animate(timestamp: number) {
    const currentTime = timestamp / 1000;
    const deltaTime = currentTime - lastFrameTime.value;
    lastFrameTime.value = currentTime;

    if (startTime.value === 0) {
        startTime.value = currentTime;
        phaseStartTime.value = currentTime;
    }

    // 计算当前阶段的运行时间
    const phaseElapsedTime = currentTime - phaseStartTime.value;

    // 更新所有粒子
    particles.value.forEach(particle => {


        // 获取基础半径
        let blend = particle.radiusController.x();

        let radius = getR(particle.initialRadius, particle.rotatingRadius, blend);

        // 计算呼吸效果 - 只有当breathController的值大于0时才应用
        const breathAmplitude = particle.breathController.x();
        if (breathAmplitude > 0) {
            // 呼吸效果：随时间波动，每个粒子有不同相位
            const breathEffect = Math.sin((currentTime - startTime.value) * 1.2 + particle.radiusOffset) * breathAmplitude;
            radius += breathEffect;
        }

        // 旋转速度根据阶段调整
        let rotationSpeed = particle.rotationSpeed;

        // 计算当前角度
        particle.angle += rotationSpeed * deltaTime * (1.1 - particle.rotateController.x()) * 15;

        // 更新粒子位置
        particle.x = Math.cos(particle.angle) * radius;
        particle.y = Math.sin(particle.angle) * radius;

        // 获取不透明度值用于CSS
        const opacity = particle.opacityController.x();

        // 使用缓存的RGB值创建颜色字符串
        const [r, g, b] = particle.rgbValues;
        particle.baseColor = `rgba(${r}, ${g}, ${b}, ${opacity})`;

        // 更新各控制器
        particle.radiusController.updateSin(currentTime);  // 使用正弦缓动更平滑
        particle.opacityController.updateSin(currentTime); // 同样使用正弦缓动
        particle.breathController.updateSin(currentTime);
        particle.rotateController.updateSin(currentTime); // 更新旋转速度

    });

    // 检查是否需要进入下一阶段
    if (!isExiting.value) {
        if (phase.value === 'entering' && phaseElapsedTime >= phaseDurations.entering) {
            phase.value = 'rotating';
            phaseStartTime.value = currentTime;

            // 为所有粒子准备旋转阶段
            particles.value.forEach(particle => {
                prepareParticleForRotation(particle, currentTime);
            });
        }
        else if (phase.value === 'rotating' && phaseElapsedTime >= phaseDurations.rotating) {
            phase.value = 'exiting';
            phaseStartTime.value = currentTime;

            // 为所有粒子准备退出阶段
            particles.value.forEach(particle => {
                prepareParticleForExit(particle, currentTime);
            });
        }
    }

    animationFrameId.value = requestAnimationFrame(animate);
}

// 开始动画
function startAnimation() {
    if (particles.value.length === 0) {
        initParticles();
    }
    startTime.value = 0;
    lastFrameTime.value = 0;
    phaseStartTime.value = 0;
    phase.value = 'entering';
    isExiting.value = false;
    animationFrameId.value = requestAnimationFrame(animate);
}

// 生命周期钩子
onMounted(() => {
    if (props.show) {
        startAnimation();
    }
});

onUnmounted(() => {
    if (animationFrameId.value !== null) {
        cancelAnimationFrame(animationFrameId.value);
    }
});
</script>


<template>
    <Transition name="fade">
        <div v-if="visible" class="loading-logo-container" :class="{ 'fade-out': !show }">
            <div class="logo-wrapper">
                <img src="../assets/npulearn.png" alt="Logo" class="logo-icon"
                    :class="{ 'fade-in': phase === 'rotating' || phase === 'exiting' }" />
                <div class="logo">
                    <div class="particles-container">
                        <div v-for="(particle, index) in particles" :key="index" class="particle" :style="{
                            transform: `translate(${particle.x}px, ${particle.y}px)`,
                            width: `${particle.size}px`,
                            height: `${particle.size}px`,
                            backgroundColor: particle.baseColor
                        }"></div>
                    </div>
                </div>
                <h1 class="app-title" :class="{ 'fade-in': phase === 'rotating' || phase === 'exiting' }">NPULearn</h1>
                <div class="loader" :class="{ 'fade-in': phase === 'rotating' || phase === 'exiting' }">
                    <div class="loader-dot"></div>
                    <div class="loader-dot"></div>
                    <div class="loader-dot"></div>
                </div>
            </div>
        </div>
    </Transition>
</template>

<style scoped>
/* 样式保持不变 */
.loading-logo-container {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    background-color: var(--bg-color);
    z-index: 9999;
    transition: opacity 0.6s ease-out, visibility 0.6s ease-out;
    opacity: 1;
    visibility: visible;
}

.loading-logo-container.fade-out {
    opacity: 0;
}

.logo-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    transform: translateY(0);
    transition: transform 0.6s cubic-bezier(0.16, 1, 0.3, 1);
}

.fade-out .logo-wrapper {
    transform: translateY(-20px);
}

.logo {
    margin-bottom: 20px;
    width: 120px;
    height: 120px;
    display: flex;
    justify-content: center;
    align-items: center;
}

.logo-icon {
    margin-bottom: 20px;
    width: 120px;
    height: 120px;
    display: flex;
    justify-content: center;
    align-items: center;
    opacity: 0;
}

.logo-icon.fade-in {
    opacity: 1;
    animation: fadeIn 0.6s ease-out;
}

.particles-container {
    position: relative;
    width: 100px;
    height: 100px;
    display: flex;
    justify-content: center;
    align-items: center;
}

.particle {
    position: absolute;
    border-radius: 50%;
    transform-origin: center;
    top: 50%;
    left: 50%;
    box-shadow: 0 0 8px rgba(79, 70, 229, 0.4);
    will-change: transform, background-color;
}


.app-title {
    font-size: 1.8rem;
    font-weight: 600;
    margin-bottom: 24px;
    color: var(--text-color);
    letter-spacing: 0.5px;
    opacity: 0;
    transform: translateY(10px);
    transition: opacity 0.8s ease-out, transform 0.8s ease-out;
}

.app-title.fade-in {
    opacity: 1;
    transform: translateY(0);
}


.loader {
    display: flex;
    gap: 8px;
    opacity: 0;
    transform: translateY(8px);
    transition: opacity 0.8s ease-out 0.2s, transform 0.8s ease-out 0.2s;
}

.loader.fade-in {
    opacity: 1;
    transform: translateY(0);
}

.loader-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background-color: var(--primary-color);
    opacity: 0.6;
}

.loader-dot:nth-child(1) {
    animation: dot-fade 1.2s 0s infinite ease-in-out;
}

.loader-dot:nth-child(2) {
    animation: dot-fade 1.2s 0.4s infinite ease-in-out;
}

.loader-dot:nth-child(3) {
    animation: dot-fade 1.2s 0.8s infinite ease-in-out;
}

@keyframes pulse {
    0% {
        transform: scale(0.95);
        opacity: 0.7;
    }

    100% {
        transform: scale(1.05);
        opacity: 1;
    }
}

@keyframes fadeIn {
    0% {
        opacity: 0;
    }

    100% {
        opacity: 1;
    }
}

@keyframes dot-fade {

    0%,
    100% {
        opacity: 0.2;
    }

    50% {
        opacity: 1;
    }
}

.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.6s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}

</style>