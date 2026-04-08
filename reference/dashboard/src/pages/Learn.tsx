import { FC, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { BookOpen, CheckCircle, Award } from 'lucide-react';
import { useWallet } from '@solana/wallet-adapter-react';
import { apiService } from '../services/api';
import BadgesSection from '../components/Badges';

interface Course {
  id: string;
  title: string;
  description: string;
  slug: string;
  content: {
    lessons: Array<{
      id: string;
      title: string;
      description: string;
      content: string;
    }>;
  };
  order_num: number;
  is_active: boolean;
}

interface Progress {
  course_id: string;
  completed: boolean;
  progress_data: {
    completed_lessons?: string[];
  };
}

const Learn: FC = () => {
  const { connected } = useWallet();
  const [courses, setCourses] = useState<Course[]>([]);
  const [progress, setProgress] = useState<Record<string, Progress>>({});
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadCourses();
    loadProgress();
  }, []);

  const loadCourses = async () => {
    try {
      const data = await apiService.getCourses();
      setCourses(data.courses || []);
    } catch (error) {
      console.error('Failed to load courses:', error);
    } finally {
      setLoading(false);
    }
  };

  const loadProgress = async () => {
    try {
      const data = await apiService.getMyProgress();
      const progressMap: Record<string, Progress> = {};
      data.forEach((p: Progress) => {
        progressMap[p.course_id] = p;
      });
      setProgress(progressMap);
    } catch (error) {
      console.log('Progress not available (user may not be logged in)');
    }
  };

  const calculateProgress = (course: Course, courseProgress?: Progress) => {
    if (!courseProgress) return 0;
    const completedLessons = courseProgress.progress_data.completed_lessons || [];
    const totalLessons = course.content.lessons.length;
    return totalLessons > 0 ? (completedLessons.length / totalLessons) * 100 : 0;
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen">
        <div className="text-lg text-slate-400">Loading courses...</div>
      </div>
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
      {/* Header */}
      <div className="text-center mb-12">
        <div className="inline-flex items-center gap-2 border border-blue-500/30 bg-blue-500/10 rounded-full px-4 py-2 mb-6">
          <Award className="text-blue-400" size={16} />
          <span className="text-xs text-blue-400 uppercase tracking-wide font-medium">
            Security Education
          </span>
        </div>
        <h1 className="text-4xl sm:text-5xl font-bold mb-4 text-white">
          Learn Security Best Practices
        </h1>
        <p className="text-lg text-slate-400 max-w-3xl mx-auto">
          Master blockchain and social media security with interactive courses designed for Solana users
        </p>
      </div>

      {/* Badges Section */}
      {connected && (
        <div className="mb-12">
          <BadgesSection />
        </div>
      )}

      {/* Courses Grid */}
      <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
        {courses.map((course) => {
          const courseProgress = progress[course.id];
          const isCompleted = courseProgress?.completed || false;
          const progressPercent = calculateProgress(course, courseProgress);

          return (
            <Link
              key={course.id}
              to={`/learn/${course.slug}`}
              className="bg-white/5 border border-white/10 rounded-lg p-6 hover:bg-white/[0.07] hover:border-blue-500/50 transition-all group"
            >
              <div className="flex items-start justify-between mb-4">
                <div className="p-3 bg-blue-500/10 rounded-lg group-hover:bg-blue-500/20 transition-colors">
                  <BookOpen className="text-blue-400" size={24} />
                </div>
                {isCompleted && (
                  <CheckCircle className="text-green-400" size={24} />
                )}
              </div>
              
              <h3 className="text-xl font-bold text-white mb-2 group-hover:text-blue-400 transition-colors">
                {course.title}
              </h3>
              
              <p className="text-sm text-slate-400 mb-4 line-clamp-2">
                {course.description || 'Learn essential security practices'}
              </p>

              <div className="text-xs text-slate-500 mb-4">
                {course.content.lessons.length} lessons
              </div>

              {courseProgress && (
                <div className="space-y-2">
                  <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-blue-500 to-purple-500 transition-all duration-500"
                      style={{ width: `${progressPercent}%` }}
                    />
                  </div>
                  <span className="text-xs text-slate-400">
                    {Math.round(progressPercent)}% Complete
                  </span>
                </div>
              )}
            </Link>
          );
        })}
      </div>

      {courses.length === 0 && (
        <div className="text-center py-12">
          <p className="text-slate-400">No courses available yet. Check back soon!</p>
        </div>
      )}
    </div>
  );
};

export default Learn;
