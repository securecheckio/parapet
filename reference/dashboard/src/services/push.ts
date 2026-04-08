// Push Notification Service

// Fetch VAPID public key from backend
async function getVapidPublicKey(): Promise<string> {
  try {
    const response = await fetch('http://localhost:3001/vapid-public-key');
    const data = await response.json();
    return data.publicKey;
  } catch (error) {
    console.error('Failed to fetch VAPID public key:', error);
    throw new Error('Could not get VAPID public key');
  }
}

function urlBase64ToUint8Array(base64String: string): Uint8Array {
  const padding = '='.repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding)
    .replace(/\-/g, '+')
    .replace(/_/g, '/');

  const rawData = window.atob(base64);
  const outputArray = new Uint8Array(rawData.length);

  for (let i = 0; i < rawData.length; ++i) {
    outputArray[i] = rawData.charCodeAt(i);
  }
  return outputArray;
}

export async function subscribeToPushNotifications(): Promise<PushSubscription | null> {
  if (!('serviceWorker' in navigator) || !('PushManager' in window)) {
    console.error('Push notifications not supported');
    return null;
  }

  try {
    // Get VAPID public key from backend
    const vapidPublicKey = await getVapidPublicKey();
    
    // Wait for service worker to be ready
    const registration = await navigator.serviceWorker.ready;
    
    // Check if already subscribed
    let subscription = await registration.pushManager.getSubscription();
    
    if (!subscription) {
      // Create new subscription
      subscription = await registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: urlBase64ToUint8Array(vapidPublicKey),
      });
      
      console.log('✅ Push subscription created');
    } else {
      console.log('✅ Already subscribed to push');
    }

    return subscription;
  } catch (error) {
    console.error('❌ Failed to subscribe to push:', error);
    return null;
  }
}

export async function sendSubscriptionToServer(subscription: PushSubscription): Promise<boolean> {
  try {
    const subscriptionJson = subscription.toJSON();
    
    const response = await fetch('http://localhost:3001/dashboard/push/subscribe', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      credentials: 'include',
      body: JSON.stringify(subscriptionJson),
    });

    if (response.ok) {
      console.log('✅ Push subscription sent to server');
      return true;
    } else {
      console.error('❌ Failed to send subscription to server:', response.status);
      return false;
    }
  } catch (error) {
    console.error('❌ Error sending subscription to server:', error);
    return false;
  }
}
