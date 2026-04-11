import { useAppStore } from '../stores/app';
import './Notifications.css';

export default function Notifications() {
  const { notifications, removeNotification } = useAppStore();

  if (notifications.length === 0) return null;

  return (
    <div class="notifications">
      {notifications.map((notif) => (
        <div
          key={notif.id}
          class={`notif notif-${notif.type} animate-slide-in`}
          onClick={() => removeNotification(notif.id)}
        >
          <span class="notif-icon">
            {notif.type === 'success' && '✓'}
            {notif.type === 'error' && '✕'}
            {notif.type === 'info' && 'ℹ'}
          </span>
          <span class="notif-message">{notif.message}</span>
        </div>
      ))}
    </div>
  );
}
