import React from 'react'
import { Space, Divider } from 'antd'

export interface ContainerProps extends React.HTMLAttributes<HTMLDivElement> {
  size?: 'sm' | 'md' | 'lg' | 'xl' | 'full'
}

export const Container: React.FC<ContainerProps> = ({ className, size = 'lg', children, ...props }) => {
  const containerStyle: React.CSSProperties = {}

  switch (size) {
    case 'sm':
      containerStyle.maxWidth = '768px'
      break
    case 'md':
      containerStyle.maxWidth = '1024px'
      break
    case 'lg':
      containerStyle.maxWidth = '1280px'
      break
    case 'xl':
      containerStyle.maxWidth = '1536px'
      break
    case 'full':
      containerStyle.maxWidth = '100%'
      break
  }

  return (
    <div
      className={className}
      style={{
        ...containerStyle,
        marginLeft: 'auto',
        marginRight: 'auto',
        paddingLeft: '1rem',
        paddingRight: '1rem',
        ...props.style,
      }}
      {...props}
    >
      {children}
    </div>
  )
}

export interface StackProps extends React.HTMLAttributes<HTMLElement> {
  spacing?: 'small' | 'middle' | 'large'
  orientation?: 'vertical' | 'horizontal'
}

export const Stack: React.FC<StackProps> = ({ className, spacing = 'middle', orientation = 'vertical', children, ...props }) => {
  const gapMap = {
    small: 8,
    middle: 16,
    large: 24,
  }

  return (
    <Space
      className={className}
      orientation={orientation}
      size={gapMap[spacing]}
      {...(props as any)}
    >
      {children}
    </Space>
  )
}

export interface SeparatorProps extends React.HTMLAttributes<HTMLDivElement> {
  orientation?: 'horizontal' | 'vertical'
  decorative?: boolean
}

export const Separator: React.FC<SeparatorProps> = ({ className, orientation = 'horizontal', decorative = true, ...props }) => {
  return (
    <Divider
      className={className}
      {...props}
    />
  )
}

export default { Container, Stack, Separator }
