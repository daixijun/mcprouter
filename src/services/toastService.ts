class ToastService {
  private static instance: ToastService
  private toastContainer: HTMLElement | null = null

  private constructor() {
    this.createToastContainer()
  }

  static getInstance(): ToastService {
    if (!ToastService.instance) {
      ToastService.instance = new ToastService()
    }
    return ToastService.instance
  }

  private createToastContainer(): void {
    // 检查是否已存在容器
    this.toastContainer = document.getElementById('toast-container')
    if (!this.toastContainer) {
      this.toastContainer = document.createElement('div')
      this.toastContainer.id = 'toast-container'
      this.toastContainer.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        z-index: 9999;
        display: flex;
        flex-direction: column;
        gap: 10px;
      `
      document.body.appendChild(this.toastContainer)
    }
  }

  private createToastElement(
    type: 'success' | 'error' | 'info',
    title: string,
    message?: string,
  ): HTMLElement {
    const toast = document.createElement('div')

    // 根据类型设置样式
    const backgroundColor = {
      success: '#10b981',
      error: '#ef4444',
      info: '#3b82f6',
    }[type]

    toast.style.cssText = `
      background-color: ${backgroundColor};
      color: white;
      padding: 12px 16px;
      border-radius: 6px;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
      min-width: 300px;
      max-width: 500px;
      transform: translateX(100%);
      transition: transform 0.3s ease-out;
      display: flex;
      flex-direction: column;
      gap: 4px;
    `

    // 标题
    const titleElement = document.createElement('div')
    titleElement.style.cssText = `
      font-weight: bold;
      font-size: 16px;
    `
    titleElement.textContent = title
    toast.appendChild(titleElement)

    // 消息内容
    if (message) {
      const messageElement = document.createElement('div')
      messageElement.style.cssText = `
        font-size: 14px;
        opacity: 0.9;
      `
      messageElement.textContent = message
      toast.appendChild(messageElement)
    }

    return toast
  }

  private showToast(
    type: 'success' | 'error' | 'info',
    title: string,
    message?: string,
  ): void {
    if (!this.toastContainer) {
      this.createToastContainer()
    }

    const toast = this.createToastElement(type, title, message)
    this.toastContainer!.appendChild(toast)

    // 触发动画
    setTimeout(() => {
      toast.style.transform = 'translateX(0)'
    }, 10)

    // 自动移除
    setTimeout(() => {
      toast.style.transform = 'translateX(100%)'
      setTimeout(() => {
        if (toast.parentNode) {
          toast.parentNode.removeChild(toast)
        }
      }, 300)
    }, 3000)
  }

  sendSuccessNotification(message: string): void {
    this.showToast('success', '成功', message)
  }

  sendErrorNotification(message: string): void {
    this.showToast('error', '错误', message)
  }

  sendInfoNotification(message: string): void {
    this.showToast('info', '信息', message)
  }
}

export default ToastService.getInstance()
